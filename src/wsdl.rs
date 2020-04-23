//! WSDL inspection helpers.

use std::collections::HashMap;
use xml::reader::{EventReader, XmlEvent};
use xmltree::Element;

#[derive(Debug)]
pub enum WsdlError {
  Parse(xmltree::ParseError),
  ElementNotFound(&'static str),
  AttributeNotFound(&'static str),
  NotAnElement,
  Empty,
}

impl From<xmltree::ParseError> for WsdlError {
    fn from(error: xmltree::ParseError) -> Self {
        WsdlError::Parse(error)
    }
}

/// WSDL operation info.
#[derive(Debug)]
pub struct Operation {
    pub url: String,
}

/// WSDL document.
#[derive(Debug)]
pub struct Wsdl {
    pub operations: HashMap<String, Operation>,
}

#[derive(Debug, Clone)]
enum SimpleType {
    String,
    Float,
    Int,
}

#[derive(Debug, Clone)]
struct ComplexType {
  fields: HashMap<String, SimpleType>,
}

#[derive(Debug, Clone)]
enum Type {
    Simple(SimpleType),
    Complex(ComplexType),
}

#[derive(Debug, Clone)]
struct Message {
  // FIXME: convert to type name
  parts: HashMap<String, String>,
}

pub fn parse(bytes: &[u8]) -> Result<Wsdl, WsdlError> {
    let mut types = HashMap::new();
    let mut messages = HashMap::new();

    //let mut operations = HashMap::new();

    let elements = Element::parse(bytes)?;
    println!("elements: {:#?}", elements);

    let types_el = elements.get_child("types").ok_or(WsdlError::ElementNotFound("types"))?
        .children.get(0).ok_or(WsdlError::Empty)?
        .as_element().ok_or(WsdlError::NotAnElement)?;
    //println!("types: {:#?}", types_el);

    for elem in types_el.children.iter().filter_map(|c| c.as_element()) {
        //println!("type: {:#?}", elem);
        let name = elem.attributes.get("name").ok_or(WsdlError::AttributeNotFound("name"))?;
        //println!("name: {:?}", name);

        let child = elem.children.get(0).ok_or(WsdlError::Empty)?
            .as_element().ok_or(WsdlError::NotAnElement)?;
        if child.name == "complexType" {
            let mut fields = HashMap::new();
            for field in child.children.get(0).ok_or(WsdlError::Empty)?
                .as_element().ok_or(WsdlError::NotAnElement)?
                .children.iter().filter_map(|c| c.as_element()) {
                let field_name = field.attributes.get("name").ok_or(WsdlError::AttributeNotFound("name"))?;
                let field_type = field.attributes.get("type").ok_or(WsdlError::AttributeNotFound("type"))?;
                //println!("field {:?} -> {:?}", field_name, field_type);

                match field_type.as_str() {
                    "string" => fields.insert(field_name.to_string(), SimpleType::String),
                    "int" => fields.insert(field_name.to_string(), SimpleType::Int),
                    "float" => fields.insert(field_name.to_string(), SimpleType::Float),
                    s => panic!("unknown simple type for field '{}' in '{}': {:?}", field_name, name, s),
                };
            }

            types.insert(name, Type::Complex(ComplexType { fields }));
        } else {
            unimplemented!("not a complex type");
        }
    }

    for message in elements.children.iter().filter_map(|c| c.as_element()).filter(|c| c.name == "message") {
        println!("message: {:#?}", message);
        let name = message.attributes.get("name").ok_or(WsdlError::AttributeNotFound("name"))?;
        let mut parts = HashMap::new();
        for c in message.children.iter().filter_map(|c| c.as_element()) {
            let part_name = c.attributes.get("name").ok_or(WsdlError::AttributeNotFound("name"))?;
            let part_element = c.attributes.get("element").ok_or(WsdlError::AttributeNotFound("element"))?;
            //FIXME: namespace
            parts.insert(part_name.to_string(), part_element.to_string());
        }

        messages.insert(name.to_string(), Message { parts });

    }
    println!("parsed types: {:?}", types);
    println!("parsed messages: {:?}", messages);
    panic!();
}

#[cfg(test)]
mod tests {
    use super::*;
    const WIKIPEDIA_WSDL: &[u8] = include_bytes!("../assets/wikipedia-example.wsdl");
    const EXAMPLE_WSDL: &[u8] = include_bytes!("../assets/example.wsdl");
    use crate::wsdl::*;

    #[test]
    fn parse_example() {
        let res = parse(EXAMPLE_WSDL);
        println!("res: {:?}", res);
        res.unwrap();
        panic!();
    }
}
