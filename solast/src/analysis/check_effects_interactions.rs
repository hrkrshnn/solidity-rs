use eth_lang_utils::ast::*;
use solidity::ast::*;
use std::{collections::HashMap, io};

struct BlockInfo {
    makes_external_call: bool,
    makes_post_external_call_assignment: bool,
    variable_bindings: HashMap<NodeID, Vec<NodeID>>,
    parent_blocks: Vec<NodeID>,
}

struct FunctionInfo {
    block_info: HashMap<NodeID, BlockInfo>,
}

struct ContractInfo {
    function_info: HashMap<NodeID, FunctionInfo>,
}

#[derive(Default)]
pub struct CheckEffectsInteractionsVisitor {
    contract_info: HashMap<NodeID, ContractInfo>,
}

impl CheckEffectsInteractionsVisitor {
    fn get_state_variable_id(
        source_units: &[SourceUnit],
        contract_definition: &ContractDefinition,
        function_info: &mut FunctionInfo,
        block_id: NodeID,
        referenced_declaration: NodeID
    ) -> Option<NodeID> {
        if contract_definition.hierarchy_contains_state_variable(source_units, referenced_declaration) {
            Some(referenced_declaration)
        } else {
            let mut state_variable_id = None;

            let block_info = function_info.block_info.get(&block_id).unwrap();

            for (&current_state_variable_id, local_variable_ids) in block_info.variable_bindings.iter() {
                if local_variable_ids.contains(&referenced_declaration) {
                    state_variable_id = Some(current_state_variable_id);
                    break;
                }
            }

            if state_variable_id.is_none() {
                for &parent_block_id in block_info.parent_blocks.iter().rev() {
                    let parent_block_info = function_info.block_info.get(&parent_block_id).unwrap();

                    for (&current_state_variable_id, local_variable_ids) in parent_block_info.variable_bindings.iter() {
                        if local_variable_ids.contains(&referenced_declaration) {
                            state_variable_id = Some(current_state_variable_id);
                            break;
                        }
                    }

                    if state_variable_id.is_some() {
                        break;
                    }
                }

                state_variable_id?;
            }
            
            state_variable_id
        }
    }

    fn check_expression(
        source_units: &[SourceUnit],
        contract_definition: &ContractDefinition,
        definition_node: &ContractDefinitionNode,
        function_info: &mut FunctionInfo,
        block_id: NodeID,
        expression: &Expression,
        source_line: usize,
    ) -> io::Result<()> {
        let mut makes_post_external_call_assignment = false;
        
        for id in expression.referenced_declarations() {
            if !contract_definition.hierarchy_contains_state_variable(source_units, id) {
                continue;
            }

            let block_info = function_info.block_info.get(&block_id).unwrap();

            for (_, ids) in block_info.variable_bindings.iter() {
                if ids.contains(&id) {
                    makes_post_external_call_assignment = true;
                    break;
                }
            }

            if makes_post_external_call_assignment {
                break;
            }

            for &parent_block_id in block_info.parent_blocks.iter() {
                let parent_block_info = function_info.block_info.get(&parent_block_id).unwrap();

                for (_, ids) in parent_block_info.variable_bindings.iter() {
                    if ids.contains(&id) {
                        makes_post_external_call_assignment = true;
                        break;
                    }
                }

                if makes_post_external_call_assignment {
                    break;
                }
            }

            if makes_post_external_call_assignment {
                break;
            }
        }

        if makes_post_external_call_assignment {
            let block_info = function_info.block_info.get_mut(&block_id).unwrap();
            block_info.makes_post_external_call_assignment = true;

            Self::print_message(contract_definition, definition_node, source_line);
        }

        Ok(())
    }

    fn print_message(
        contract_definition: &ContractDefinition,
        definition_node: &ContractDefinitionNode,
        source_line: usize,
    ) {
        println!(
            "\t{} ignores the Check-Effects-Interactions pattern",
            contract_definition.definition_node_location(source_line, definition_node),
        );
    }
}

impl AstVisitor for CheckEffectsInteractionsVisitor {
    fn visit_contract_definition<'a>(&mut self, context: &mut ContractDefinitionContext<'a>) -> io::Result<()> {
        self.contract_info.entry(context.contract_definition.id).or_insert_with(|| ContractInfo {
            function_info: HashMap::new(),
        });

        Ok(())
    }

    fn visit_function_definition<'a>(&mut self, context: &mut FunctionDefinitionContext<'a>) -> io::Result<()> {
        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();

        contract_info.function_info.entry(context.function_definition.id).or_insert_with(|| FunctionInfo {
            block_info: HashMap::new(),
        });

        Ok(())
    }

    fn visit_modifier_definition<'a>(&mut self, context: &mut ModifierDefinitionContext<'a>) -> io::Result<()> {
        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();

        contract_info.function_info.entry(context.modifier_definition.id).or_insert_with(|| FunctionInfo {
            block_info: HashMap::new(),
        });

        Ok(())
    }

    fn visit_block<'a, 'b>(&mut self, context: &mut BlockContext<'a, 'b>) -> io::Result<()> {
        let definition_id = match context.definition_node {
            ContractDefinitionNode::FunctionDefinition(FunctionDefinition { id, .. }) => id,
            ContractDefinitionNode::ModifierDefinition(ModifierDefinition { id, .. }) => id,
            _ => return Ok(())
        };

        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();
        let function_info = contract_info.function_info.get_mut(definition_id).unwrap();

        function_info.block_info.entry(context.block.id).or_insert_with(|| BlockInfo {
            makes_external_call: false,
            makes_post_external_call_assignment: false,
            variable_bindings: HashMap::new(),
            parent_blocks: context.blocks.iter().map(|&block| block.id).collect(),
        });

        Ok(())
    }

    fn visit_variable_declaration_statement<'a, 'b>(&mut self, context: &mut VariableDeclarationStatementContext<'a, 'b>) -> io::Result<()> {
        if context.blocks.is_empty() {
            return Ok(())
        }
        
        let definition_id = match context.definition_node {
            ContractDefinitionNode::FunctionDefinition(FunctionDefinition { id, .. }) => id,
            ContractDefinitionNode::ModifierDefinition(ModifierDefinition { id, .. }) => id,
            _ => return Ok(())
        };

        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();
        let function_info = contract_info.function_info.get_mut(definition_id).unwrap();
        let block_id = context.blocks.last().unwrap().id;
        
        //
        // Only store bindings for storage reference variables
        //

        match context.variable_declaration_statement.initial_value.as_ref().map(Expression::root_expression) {
            Some(Some(&Expression::Identifier(Identifier { referenced_declaration, .. }))) => {
                assert!(context.variable_declaration_statement.declarations.len() == 1);

                let local_variable_id = match context.variable_declaration_statement.declarations.first().unwrap() {
                    &Some(VariableDeclaration {
                        storage_location: StorageLocation::Storage,
                        id,
                        ..
                    }) => id,

                    _ => return Ok(())
                };

                let state_variable_id = match Self::get_state_variable_id(
                    context.source_units,
                    context.contract_definition,
                    function_info,
                    block_id,
                    referenced_declaration
                ) {
                    Some(id) => id,
                    None => return Ok(())
                };

                let block_info = function_info.block_info.get_mut(&block_id).unwrap();

                let variable_bindings = block_info.variable_bindings.entry(state_variable_id).or_insert_with(Vec::new);

                if !variable_bindings.contains(&local_variable_id) {
                    variable_bindings.push(local_variable_id);
                }
            }

            Some(Some(Expression::TupleExpression(TupleExpression { components, .. }))) => {
                assert!(components.len() == context.variable_declaration_statement.declarations.len());

                for (i, component) in components.iter().enumerate() {
                    if component.is_none() {
                        continue;
                    }
                    
                    let referenced_declaration = match component.as_ref().unwrap().root_expression() {
                        Some(&Expression::Identifier(Identifier { referenced_declaration, .. })) => referenced_declaration,
                        _ => continue
                    };
                    
                    let local_variable_id = match context.variable_declaration_statement.declarations.get(i).unwrap() {
                        &Some(VariableDeclaration {
                            storage_location: StorageLocation::Storage,
                            id,
                            ..
                        }) => id,

                        _ => continue
                    };

                    let state_variable_id = match Self::get_state_variable_id(
                        context.source_units,
                        context.contract_definition,
                        function_info,
                        block_id,
                        referenced_declaration
                    ) {
                        Some(id) => id,
                        None => continue
                    };

                    let block_info = function_info.block_info.get_mut(&block_id).unwrap();

                    let variable_bindings = block_info.variable_bindings.entry(state_variable_id).or_insert_with(Vec::new);

                    if !variable_bindings.contains(&local_variable_id) {
                        variable_bindings.push(local_variable_id);
                    }
                }
            }

            _ => {}
        }

        Ok(())
    }

    fn visit_identifier<'a, 'b>(&mut self, context: &mut IdentifierContext<'a, 'b>) -> io::Result<()> {
        if context.blocks.is_empty() {
            return Ok(())
        }

        let definition_id = match context.definition_node {
            ContractDefinitionNode::FunctionDefinition(FunctionDefinition { id, .. }) => id,
            ContractDefinitionNode::ModifierDefinition(ModifierDefinition { id, .. }) => id,
            _ => return Ok(())
        };

        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();
        let function_info = contract_info.function_info.get_mut(definition_id).unwrap();
        let block_info = function_info.block_info.get_mut(&context.blocks.last().unwrap().id).unwrap();

        //
        // Don't check the identifier if the current block is already marked
        //

        if block_info.makes_external_call {
            return Ok(())
        }

        //
        // Check if the identifier references an external function
        //

        for source_unit in context.source_units.iter() {
            if let Some(FunctionDefinition {
                visibility: Visibility::External,
                ..
            }) = source_unit.function_definition(context.identifier.referenced_declaration) {
                block_info.makes_external_call = true;
                break;
            }
        }

        Ok(())
    }

    fn visit_member_access<'a, 'b>(&mut self, context: &mut MemberAccessContext<'a, 'b>) -> io::Result<()> {
        if context.blocks.is_empty() {
            return Ok(())
        }
        
        let definition_id = match context.definition_node {
            ContractDefinitionNode::FunctionDefinition(FunctionDefinition { id, .. }) => id,
            ContractDefinitionNode::ModifierDefinition(ModifierDefinition { id, .. }) => id,
            _ => return Ok(())
        };

        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();
        let function_info = contract_info.function_info.get_mut(definition_id).unwrap();
        let block_info = function_info.block_info.get_mut(&context.blocks.last().unwrap().id).unwrap();

        //
        // Don't check the member access if the current block is already marked
        //

        if block_info.makes_external_call {
            return Ok(())
        }

        //
        // Check if the member access references an external function
        //

        if let Some(id) = context.member_access.referenced_declaration {
            for source_unit in context.source_units.iter() {
                if let Some(FunctionDefinition {
                    visibility: Visibility::External,
                    ..
                }) = source_unit.function_definition(id) {
                    block_info.makes_external_call = true;
                    break;
                }
            }
        }

        Ok(())
    }

    fn visit_assignment<'a, 'b>(&mut self, context: &mut AssignmentContext<'a, 'b>) -> io::Result<()> {
        let definition_id = match context.definition_node {
            &ContractDefinitionNode::FunctionDefinition(FunctionDefinition { id, .. })
            | &ContractDefinitionNode::ModifierDefinition(ModifierDefinition { id, .. }) => id,
            _ => return Ok(())
        };

        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();
        let function_info = contract_info.function_info.get_mut(&definition_id).unwrap();
        let block_id = context.blocks.last().unwrap().id;
        let block_info = function_info.block_info.get(&block_id).unwrap();

        //
        // Don't check the assignment if the current block is already marked
        //

        if block_info.makes_post_external_call_assignment {
            return Ok(())
        }

        //
        // Don't check for post-call assignments if the current scope doesn't make an external call
        //
        
        let mut makes_external_call = block_info.makes_external_call;

        if !makes_external_call {
            for &parent_block_id in block_info.parent_blocks.iter().rev() {
                if let Some(BlockInfo {
                    makes_external_call: true,
                    ..
                }) = function_info.block_info.get(&parent_block_id) {
                    makes_external_call = true;
                    break;
                }
            }

            if !makes_external_call {
                return Ok(())
            }
        }

        //
        // Check for post external call state variable assignments
        //

        Self::check_expression(
            context.source_units,
            context.contract_definition,
            context.definition_node,
            function_info,
            block_id,
            context.assignment.left_hand_side.as_ref(),
            context.current_source_unit.source_line(context.assignment.src.as_str())?
        )
    }
    
    fn visit_unary_operation<'a, 'b>(&mut self, context: &mut UnaryOperationContext<'a, 'b>) -> io::Result<()> {
        let definition_id = match context.definition_node {
            &ContractDefinitionNode::FunctionDefinition(FunctionDefinition { id, .. })
            | &ContractDefinitionNode::ModifierDefinition(ModifierDefinition { id, .. }) => id,
            _ => return Ok(())
        };

        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();
        let function_info = contract_info.function_info.get_mut(&definition_id).unwrap();
        let block_id = context.blocks.last().unwrap().id;
        let block_info = function_info.block_info.get(&block_id).unwrap();

        //
        // Don't check the unary operation if the current block is already marked
        //

        if block_info.makes_post_external_call_assignment {
            return Ok(())
        }

        //
        // Don't check the unary operation if the current scope doesn't make an external call
        //
        
        let mut makes_external_call = block_info.makes_external_call;

        if !makes_external_call {
            for &parent_block_id in block_info.parent_blocks.iter().rev() {
                if let Some(BlockInfo {
                    makes_external_call: true,
                    ..
                }) = function_info.block_info.get(&parent_block_id) {
                    makes_external_call = true;
                    break;
                }
            }

            if !makes_external_call {
                return Ok(())
            }
        }

        //
        // Check for post external call unary operations
        //
        
        Self::check_expression(
            context.source_units,
            context.contract_definition,
            context.definition_node,
            function_info,
            block_id,
            context.unary_operation.sub_expression.as_ref(),
            context.current_source_unit.source_line(context.unary_operation.src.as_str())?
        )
    }

    fn visit_function_call<'a, 'b>(&mut self, context: &mut FunctionCallContext<'a, 'b>) -> io::Result<()> {
        if context.blocks.is_empty() {
            return Ok(())
        }

        let definition_id = match context.definition_node {
            &ContractDefinitionNode::FunctionDefinition(FunctionDefinition { id, .. })
            | &ContractDefinitionNode::ModifierDefinition(ModifierDefinition { id, .. }) => id,
            _ => return Ok(())
        };

        let contract_info = self.contract_info.get_mut(&context.contract_definition.id).unwrap();
        let function_info = contract_info.function_info.get_mut(&definition_id).unwrap();
        let block_id = context.blocks.last().unwrap().id;
        let block_info = function_info.block_info.get(&block_id).unwrap();

        //
        // Don't check the function call if the current block is already marked
        //

        if block_info.makes_post_external_call_assignment {
            return Ok(())
        }

        //
        // Don't check for mutative function calls if the current scope doesn't make an external call
        //
        
        let mut makes_external_call = block_info.makes_external_call;

        if !makes_external_call {
            for &parent_block_id in block_info.parent_blocks.iter().rev() {
                if let Some(BlockInfo {
                    makes_external_call: true,
                    ..
                }) = function_info.block_info.get(&parent_block_id) {
                    makes_external_call = true;
                    break;
                }
            }

            if !makes_external_call {
                return Ok(())
            }
        }

        //
        // Check for post external call mutative function calls
        //
        
        let expression = match context.function_call.expression.as_ref() {
            Expression::MemberAccess(MemberAccess {
                referenced_declaration: None,
                member_name,
                expression,
                ..
            }) if member_name == "push" || member_name == "pop" => expression.as_ref(),

            _ => return Ok(())
        };

        Self::check_expression(
            context.source_units,
            context.contract_definition,
            context.definition_node,
            function_info,
            block_id,
            expression,
            context.current_source_unit.source_line(context.function_call.src.as_str())?
        )
    }
}
