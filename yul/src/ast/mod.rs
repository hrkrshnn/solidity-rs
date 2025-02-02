use eth_lang_utils::ast::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(untagged)]
pub enum ExternalReference {
    Untagged(ExternalReferenceData),
    Tagged(HashMap<String, ExternalReferenceData>),
}

#[derive(Clone, Debug, Eq, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalReferenceData {
    declaration: NodeID,
    is_offset: bool,
    is_slot: bool,
    src: String,
    value_size: NodeID,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(untagged)]
pub enum YulExpression {
    YulLiteral(YulLiteral),
    YulIdentifier(YulIdentifier),
    YulFunctionCall(YulFunctionCall),

    #[serde(rename_all = "camelCase")]
    UnhandledYulExpression {
        node_type: String,
        src: Option<String>,
        id: Option<NodeID>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulLiteral {
    pub kind: YulLiteralKind,
    pub value: Option<String>,
    pub hex_value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum YulLiteralKind {
    Bool,
    Number,
    String,
    HexString,
    Address,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulIdentifier {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulFunctionCall {
    pub function_name: YulIdentifier,
    pub arguments: Vec<YulExpression>,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulBlock {
    pub statements: Vec<YulStatement>,
}

pub struct YulBlockContext<'a, 'b> {
    pub yul_blocks: &'b mut Vec<&'a YulBlock>,
    pub yul_block: &'a YulBlock,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(untagged)]
pub enum YulStatement {
    YulIf(YulIf),
    YulSwitch(YulSwitch),
    YulAssignment(YulAssignment),
    YulVariableDeclaration(YulVariableDeclaration),
    YulExpressionStatement(YulExpressionStatement),

    #[serde(rename_all = "camelCase")]
    UnhandledYulStatement {
        node_type: String,
        src: Option<String>,
        id: Option<NodeID>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulIf {
    pub condition: YulExpression,
    pub body: YulBlock,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulSwitch {
    pub cases: Vec<YulCase>,
    pub expression: YulExpression,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulCase {
    pub body: YulBlock,
    pub value: YulExpression,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulAssignment {
    pub value: YulExpression,
    pub variable_names: Vec<YulIdentifier>,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulVariableDeclaration {
    pub value: YulExpression,
    pub variables: Vec<YulTypedName>,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulTypedName {
    pub r#type: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct YulExpressionStatement {
    pub expression: YulExpression,
}
