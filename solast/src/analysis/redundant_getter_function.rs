use solidity::ast::*;
use std::io;

pub struct RedundantGetterFunctionVisitor;

impl RedundantGetterFunctionVisitor {
    fn print_message(
        &mut self,
        contract_definition: &ContractDefinition,
        definition_node: &ContractDefinitionNode,
        variable_declaration: &VariableDeclaration,
        source_line: usize,
    ) {
        println!(
            "\t{} is a redundant getter function for the {} `{}.{}` state variable",
            contract_definition.definition_node_location(source_line, definition_node),
            variable_declaration.visibility,
            contract_definition.name,
            variable_declaration.name,
        );
    }
}

impl AstVisitor for RedundantGetterFunctionVisitor {
    fn visit_function_definition<'a>(&mut self, context: &mut FunctionDefinitionContext<'a>) -> io::Result<()> {
        if context.function_definition.name.is_empty() || context.function_definition.body.is_none() {
            return Ok(());
        }

        if context.function_definition.return_parameters.parameters.len() != 1 {
            return Ok(());
        }

        if context.function_definition.visibility != Visibility::Public {
            return Ok(());
        }

        let statements = context.function_definition
            .body
            .as_ref()
            .unwrap()
            .statements
            .as_slice();

        if statements.len() != 1 {
            return Ok(());
        }

        let return_statement = match &statements[0] {
            Statement::Return(return_statement) => return_statement,
            _ => return Ok(()),
        };

        let variable_declaration = match return_statement.expression.as_ref() {
            Some(Expression::Identifier(identifier)) => {
                match context.contract_definition.variable_declaration(identifier.referenced_declaration) {
                    Some(variable_declaration) => variable_declaration,
                    None => return Ok(()),
                }
            }
            _ => return Ok(()),
        };

        if (variable_declaration.name != context.function_definition.name)
            && !(variable_declaration.name.starts_with('_')
                && variable_declaration.name[1..] == context.function_definition.name)
        {
            return Ok(());
        }

        self.print_message(
            context.contract_definition,
            context.definition_node,
            variable_declaration,
            context.current_source_unit.source_line(context.function_definition.src.as_str())?,
        );

        Ok(())
    }
}
