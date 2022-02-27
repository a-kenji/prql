use super::utils::*;
use anyhow::{anyhow, Result};

use serde::{Deserialize, Serialize};

// Idents are generally columns
pub type Ident = String;
pub type Items = Vec<Item>;
pub type Idents = Vec<Ident>;
pub type Pipeline = Vec<Transformation>;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Item {
    Transformation(Transformation),
    // remove?
    Ident(Ident),
    String(String),
    Raw(String),
    Assign(Assign),
    NamedArg(NamedArg),
    Query(Items),
    Pipeline(Pipeline),
    // Holds Item-s directly if a list entry is a single item, otherwise holds
    // Item::Items. This is less verbose than always having Item::Items.
    List(Items),
    // Holds Items / Terms, not including separators like `+`.
    // (possibly rename to Terms)
    Items(Items),
    // Holds any Items.
    // (possibly rename to Items)
    Expr(Items),
    Idents(Idents),
    Function(Function),
    Table(Table),
    SString(Vec<SStringItem>),
    // Anything not yet implemented.
    Todo(String),
}

/// Transformation is currently used for a) each transformation in a pipeline
/// and b) a normal function call.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
// We probably want to implement some of these as Structs rather than just
// `Items`
pub enum Transformation {
    From(Items),
    Select(Items),
    Filter(Filter),
    Derive(Vec<Assign>),
    Aggregate {
        by: Items,
        calcs: Vec<Item>,
        assigns: Vec<Assign>,
    },
    Sort(Items),
    Take(i64),
    Join(Items),
    Func(FuncCall),
}

impl Transformation {
    /// Returns the name of the transformation.
    pub fn name(&self) -> &str {
        match self {
            Transformation::From(_) => "from",
            Transformation::Select(_) => "select",
            Transformation::Filter(_) => "filter",
            Transformation::Derive(_) => "derive",
            Transformation::Aggregate { .. } => "aggregate",
            Transformation::Sort(_) => "sort",
            Transformation::Take(_) => "take",
            Transformation::Join(_) => "join",
            Transformation::Func(FuncCall { name, .. }) => name,
        }
    }
}

// This is a function definition.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: Ident,
    pub args: Vec<Ident>,
    pub body: Items,
}

// This is a function call.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct FuncCall {
    pub name: String,
    pub args: Items,
    pub named_args: Vec<NamedArg>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: Ident,
    pub pipeline: Pipeline,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct NamedArg {
    pub name: Ident,
    pub arg: Box<Item>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Assign {
    pub lvalue: Ident,
    pub rvalue: Box<Item>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum SStringItem {
    String(String),
    Expr(Item),
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Filter(pub Items);

impl Item {
    /// Either provide a Vec with the contents of List / Item, or puts a scalar
    /// into a Vec. This is useful when we either have a scalar or a list, and
    /// want to only have to handle a single type.
    pub fn into_items(self) -> Vec<Item> {
        match self {
            Item::List(items) | Item::Items(items) | Item::Expr(items) => items,
            _ => vec![self],
        }
    }

    /// The scalar version of `into_items`. It's recursive, so will return the
    /// lowest possible single item.
    pub fn into_item(self) -> Item {
        match self {
            Item::List(ref items) | Item::Items(ref items) | Item::Expr(ref items) => {
                if items.len() == 1 {
                    items[0].clone().into_item()
                } else {
                    self
                }
            }
            _ => self,
        }
    }

    /// The scalar version of `into_items`. It's recursive, so will return the
    /// most granular possible single item.
    pub fn as_item(&self) -> &Item {
        match self {
            Item::List(items) | Item::Items(items) | Item::Expr(items) => {
                items.only().map(|item| item.as_item()).unwrap_or(self)
            }
            _ => self,
        }
    }

    // /// Returns the same type
    // pub fn into_unnested(&self) -> Item {
    //     match self {
    //         Item::List(items) | Item::Items(items) | Item::Expr(items) => {
    //         }
    //         _ => self.clone(),
    //     }
    // }
    // }

    // We could expand these with (but it will add lots of methods...)
    // https://crates.io/crates/enum-as-inner?
    pub fn as_ident(&self) -> Result<&Ident> {
        match self {
            Item::Ident(ident) => Ok(ident),
            // TODO: Hack; ideally remove.
            Item::Idents(ident) => {
                if ident.len() == 1 {
                    Ok(&ident[0])
                } else {
                    Err(anyhow!(
                        "Expected 1 ident, got {}; {:?}",
                        ident.len(),
                        ident
                    ))
                }
            }
            _ => Err(anyhow!("Expected an Ident, got {:?}", self)),
        }
    }
    pub fn as_items(&self) -> Result<&Vec<Item>> {
        if let Item::Items(items) = self {
            Ok(items)
        } else {
            Err(anyhow!("Expected Item::Items, got {:?}", self))
        }
    }
    pub fn as_named_arg(&self) -> Result<&NamedArg> {
        if let Item::NamedArg(named_arg) = self {
            Ok(named_arg)
        } else {
            Err(anyhow!("Expected Item::NamedArg, got {:?}", self))
        }
    }
    pub fn as_assign(&self) -> Result<&Assign> {
        if let Item::Assign(assign) = self {
            Ok(assign)
        } else {
            Err(anyhow!("Expected Item::Assign, got {:?}", self))
        }
    }
    pub fn as_raw(&self) -> Result<&String> {
        if let Item::Raw(raw) = self {
            Ok(raw)
        } else {
            Err(anyhow!("Expected Item::Raw, got {:?}", self))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_as_item() {
        let atom = Item::Ident("a".to_string());

        // Gets the single item through one level of nesting.
        let item = Item::List(vec![atom.clone()]);
        assert_eq!(item.as_item(), &atom);

        // No change when it's the same.
        let item = atom.clone();
        assert_eq!(item.as_item(), &item);

        // No change when there are two items in the `items`.
        let item = Item::Items(vec![atom.clone(), atom.clone()]);
        assert_eq!(item.as_item(), &item);

        // Gets the single item through two levels of nesting.
        let item = Item::Items(vec![Item::Items(vec![atom.clone()])]);
        assert_eq!(item.as_item(), &atom);
    }
}