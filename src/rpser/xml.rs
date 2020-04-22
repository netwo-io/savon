//! Helper trait to deal with XML Element tree.

use chrono::offset::Utc;
use chrono::{DateTime, ParseError};
use std::collections::HashMap;
use std::num::ParseIntError;
use xmltree::Element;

#[derive(Debug, PartialEq)]
pub enum Error {
    /// Element was expected at path, but was not found.
    NotFoundAtPath { path: Vec<String> },
    /// Expected element to contain children.
    ExpectedNotEmpty { parent: String },
    /// Expected to find element with specified type.
    ExpectedElementWithType {
        name: String,
        expected_type: String,
        given: Option<String>,
    },
    /// Can't parse received element.
    ParseIntError { name: String, inner: ParseIntError },
    /// Can't parse received element.
    ParseDateTimeError { name: String, inner: ParseError },
}

/// Helper trait for building `xmltree::Element`.
///
/// Such convenience methods were not available in `xmltree::Element`, so they are added
/// as a trait. To use them, you need to use this `BuildElement` trait.
///
/// ## Example
///
/// ```rust
/// extern crate xmltree;
/// extern crate confluence;
///
/// use xmltree::Element;
/// use confluence::rpser::xml::BuildElement;
///
/// fn main() {
///     assert_eq!(
///         Element::node("tag")
///             .with_child(
///                 Element::node("hello").with_text("world")
///             )
///             .to_string(),
///         "<?xml version=\"1.0\" encoding=\"UTF-8\"?><tag><hello>world</hello></tag>"
///     );
/// }
/// ```
pub trait BuildElement {
    /// The missing `clone` implementation for `xmltree::Element`.
    fn cloned(&self) -> Self;
    /// Create empty node.
    fn node<S>(name: S) -> Self
    where
        S: Into<String>;
    /// Modify node's name.
    fn with_name<S>(self, name: S) -> Self
    where
        S: Into<String>;
    /// Modify node's text.
    fn with_text<S>(self, text: S) -> Self
    where
        S: Into<String>;
    /// Add attribute.
    fn with_attr<KS, VS>(self, key: KS, value: VS) -> Self
    where
        KS: Into<String>,
        VS: Into<String>;
    /// Add child.
    fn with_child(self, child: Self) -> Self;
    /// Add children.
    fn with_children<I>(self, children: I) -> Self
    where
        Self: Sized,
        I: IntoIterator<Item = Self>;
    /// Add children from iterator.
    fn with_children_from_iter<'r, I>(self, children: I) -> Self
    where
        Self: 'r + Sized,
        I: Iterator<Item = &'r Self>;
    /// Convert to string (xml).
    fn to_string(&self) -> String;

    /// Descend into specified child element, destroying the parent.
    fn descend(self, path: &[&str]) -> Result<Element, Error>;

    /// Descend into first child element, destroying the parent.
    fn descend_first(self) -> Result<Element, Error>;

    /// Get clone of child element at path.
    fn get_at_path(&self, path: &[&str]) -> Result<Element, Error>;

    /// Extract the value of `long` type from the text.
    fn as_long(&self) -> Result<i64, Error>;

    /// Extract the value of `int` type from the text.
    fn as_int(&self) -> Result<i32, Error>;

    /// Extract the value of `boolean` type from the text.
    fn as_boolean(&self) -> Result<bool, Error>;

    /// Extract the value of `string` type from the text.
    fn as_string(&self) -> Result<String, Error>;

    /// Extract the value of `DateTime` type from the text.
    fn as_datetime(&self) -> Result<DateTime<Utc>, Error>;
}

impl BuildElement for Element {
    fn cloned(&self) -> Self {
        Element {
            name: self.name.clone(),
            attributes: self.attributes.clone(),
            children: self.children.iter().map(|child| child.cloned()).collect(),
            text: self.text.clone(),
            namespace: self.namespace.clone(),
            namespaces: self.namespaces.clone(),
            prefix: self.prefix.clone(),
        }
    }

    fn node<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Element {
            name: name.into(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text: None,
            namespace: None,
            namespaces: None,
            prefix: None,
        }
    }

    fn with_name<S>(mut self, name: S) -> Self
    where
        S: Into<String>,
    {
        self.name = name.into();
        self
    }

    fn with_text<S>(mut self, text: S) -> Self
    where
        S: Into<String>,
    {
        self.text = Some(text.into());
        self
    }

    fn with_attr<KS, VS>(mut self, key: KS, value: VS) -> Self
    where
        KS: Into<String>,
        VS: Into<String>,
    {
        self.attributes.insert(key.into(), value.into());
        self
    }

    fn with_child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }

    fn with_children<I>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        self.children.extend(children);
        self
    }

    fn with_children_from_iter<'r, I>(mut self, children: I) -> Self
    where
        I: Iterator<Item = &'r Self>,
    {
        for child in children {
            self.children.push(child.cloned());
        }
        self
    }

    fn to_string(&self) -> String {
        let mut xml = Vec::new();
        self.write(&mut xml)
            .unwrap_or_else(|e| println!("Unable to write xml: {:?}", e));
        String::from_utf8_lossy(&xml).into_owned()
    }

    fn descend(self, path: &[&str]) -> Result<Element, Error> {
        if path.is_empty() {
            Ok(self)
        } else {
            for child in self.children {
                if child.name == path[0] {
                    return match child.descend(&path[1..]) {
                        Ok(element) => Ok(element),
                        Err(Error::NotFoundAtPath {
                            path: mut error_path,
                        }) => {
                            error_path.insert(0, path[0].into());
                            Err(Error::NotFoundAtPath { path: error_path })
                        }
                        _ => unreachable!("descend should only return NotFoundAtPath error"),
                    };
                }
            }
            Err(Error::NotFoundAtPath {
                path: vec![path[0].into()],
            })
        }
    }

    fn descend_first(mut self) -> Result<Element, Error> {
        if self.children.is_empty() {
            Err(Error::ExpectedNotEmpty { parent: self.name })
        } else {
            Ok(self.children.remove(0))
        }
    }

    fn get_at_path(&self, path: &[&str]) -> Result<Element, Error> {
        if path.is_empty() {
            Ok(self.cloned())
        } else {
            for child in &self.children {
                if child.name == path[0] {
                    return match child.get_at_path(&path[1..]) {
                        Ok(element) => Ok(element),
                        Err(Error::NotFoundAtPath {
                            path: mut error_path,
                        }) => {
                            error_path.insert(0, path[0].into());
                            Err(Error::NotFoundAtPath { path: error_path })
                        }
                        _ => unreachable!("descend should only return NotFoundAtPath error"),
                    };
                }
            }
            Err(Error::NotFoundAtPath {
                path: vec![path[0].into()],
            })
        }
    }

    fn as_int(&self) -> Result<i32, Error> {
        let text = try!(get_typed_string(self, "int"));
        Ok(match text.parse() {
            Ok(ref value) => *value,
            Err(e) => {
                return Err(Error::ParseIntError {
                    name: self.name.clone(),
                    inner: e,
                });
            }
        })
    }

    fn as_long(&self) -> Result<i64, Error> {
        let text = try!(get_typed_string(self, "long"));
        Ok(match text.parse() {
            Ok(ref value) => *value,
            Err(e) => {
                return Err(Error::ParseIntError {
                    name: self.name.clone(),
                    inner: e,
                });
            }
        })
    }

    fn as_string(&self) -> Result<String, Error> {
        get_typed_string(self, "string")
    }

    fn as_datetime(&self) -> Result<DateTime<Utc>, Error> {
        let text = try!(get_typed_string(self, "dateTime"));
        Ok(match text.parse::<DateTime<Utc>>() {
            Ok(ref value) => *value,
            Err(e) => {
                return Err(Error::ParseDateTimeError {
                    name: self.name.clone(),
                    inner: e,
                });
            }
        })
    }

    fn as_boolean(&self) -> Result<bool, Error> {
        let text = try!(get_typed_string(self, "boolean"));
        Ok(text == "true")
    }
}

fn get_typed_string(element: &Element, value_type: &str) -> Result<String, Error> {
    Ok(match (element.attributes.get("type"), &element.text) {
        (Some(value), &Some(ref text)) if value.ends_with(value_type) => text.clone(),
        (other_type, _) => {
            return Err(Error::ExpectedElementWithType {
                name: element.name.clone(),
                expected_type: ["*:", value_type].concat(),
                given: other_type.cloned(),
            });
        }
    })
}
