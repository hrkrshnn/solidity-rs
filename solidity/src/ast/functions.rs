use crate::ast::*;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd, Eq, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub enum FunctionKind {
    Constructor,
    Function,
    Receive,
    Fallback,
}

impl Display for FunctionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", format!("{:?}", self).to_lowercase()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParameterList {
    pub parameters: Vec<VariableDeclaration>,
    pub src: String,
    pub id: NodeID,
}

impl Display for ParameterList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;

        for (i, parameter) in self.parameters.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }

            f.write_fmt(format_args!("{}", parameter))?;
        }

        f.write_str(")")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverrideSpecifier {
    pub overrides: Vec<IdentifierPath>,
    pub src: String,
    pub id: NodeID,
}

impl Display for OverrideSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("override")?;

        if !self.overrides.is_empty() {
            f.write_str("(")?;

            for (i, identifier_path) in self.overrides.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }

                f.write_fmt(format_args!("{}", identifier_path))?;
            }

            f.write_str(")")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDefinition {
    pub base_functions: Option<Vec<NodeID>>,
    pub body: Option<Block>,
    pub documentation: Option<Documentation>,
    pub function_selector: Option<String>,
    pub implemented: bool,
    pub kind: FunctionKind,
    pub modifiers: Vec<ModifierInvocation>,
    pub name: String,
    pub name_location: Option<String>,
    pub overrides: Option<OverrideSpecifier>,
    pub parameters: ParameterList,
    pub return_parameters: ParameterList,
    pub scope: NodeID,
    pub state_mutability: StateMutability,
    pub super_function: Option<NodeID>,
    pub r#virtual: Option<bool>,
    pub visibility: Visibility,
    pub src: String,
    pub id: NodeID,
}

impl FunctionDefinition {
    pub fn get_assigned_return_variables(
        &self,
        expression: &Expression,
    ) -> Vec<NodeID> {
        let mut ids = vec![];

        match expression {
            Expression::Identifier(identifier) => {
                if self.return_parameters.parameters.iter().find(|p| p.id == identifier.referenced_declaration).is_some() {
                    ids.push(identifier.referenced_declaration);
                }
            }

            Expression::Assignment(assignment) => {
                ids.extend(self.get_assigned_return_variables(assignment.left_hand_side.as_ref()));
            }

            Expression::IndexAccess(index_access) => {
                ids.extend(self.get_assigned_return_variables(index_access.base_expression.as_ref()));
            }

            Expression::IndexRangeAccess(index_range_access) => {
                ids.extend(self.get_assigned_return_variables(index_range_access.base_expression.as_ref()));
            }

            Expression::MemberAccess(member_access) => {
                ids.extend(self.get_assigned_return_variables(member_access.expression.as_ref()));
            }

            Expression::TupleExpression(tuple_expression) => {
                for component in tuple_expression.components.iter() {
                    if let Some(component) = component {
                        ids.extend(self.get_assigned_return_variables(component));
                    }
                }
            }

            _ => (),
        }

        ids
    }
}

impl Display for FunctionDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.kind))?;

        if !self.name.is_empty() {
            f.write_fmt(format_args!(" {}", self.name))?;
        }

        f.write_fmt(format_args!("{} {}", self.parameters, self.visibility))?;
        
        if self.state_mutability != StateMutability::NonPayable {
            f.write_fmt(format_args!(" {}", self.state_mutability))?;
        }

        if let Some(true) = self.r#virtual {
            f.write_str(" virtual")?;
        }

        if let Some(overrides) = self.overrides.as_ref() {
            f.write_fmt(format_args!(" {}", overrides))?;
        }

        for modifier in self.modifiers.iter() {
            f.write_fmt(format_args!(" {}", modifier))?;
        }

        if !self.return_parameters.parameters.is_empty() {
            f.write_fmt(format_args!(" returns {}", self.return_parameters))?;
        }

        match self.body.as_ref() {
            Some(body) => f.write_fmt(format_args!(" {}", body)),
            None => f.write_str(";"),
        }
    }
}

pub struct FunctionDefinitionContext<'a> {
    pub source_units: &'a [SourceUnit],
    pub current_source_unit: &'a SourceUnit,
    pub contract_definition: &'a ContractDefinition,
    pub definition_node: &'a ContractDefinitionNode,
    pub function_definition: &'a FunctionDefinition,
}
