//! WSDL inspection helpers.

use std::collections::HashMap;
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

/// WSDL document.
#[derive(Debug)]
pub struct Wsdl {
    pub name: String,
    pub types: HashMap<String, Type>,
    pub messages: HashMap<String, Message>,
    pub operations: HashMap<String, Operation>,
}

#[derive(Debug, Clone)]
pub enum SimpleType {
    Boolean,
    String,
    Float,
    Int,
    DateTime,
    Complex(String),
}

#[derive(Debug, Clone)]
pub enum Occurence {
  Unbounded,
  Num(u32),
}

#[derive(Debug, Clone, Default)]
pub struct TypeAttribute {
  pub nillable: bool,
  pub min_occurs: Option<Occurence>,
  pub max_occurs: Option<Occurence>,
}

#[derive(Debug, Clone)]
pub struct ComplexType {
  pub fields: HashMap<String, (TypeAttribute, SimpleType)>,
}

#[derive(Debug, Clone)]
pub enum Type {
  Simple(SimpleType),
  Complex(ComplexType),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub part_name: String,
    pub part_element: String,
}

#[derive(Debug)]
pub struct Operation {
    pub name: String,
    pub input: Option<String>,
    pub output: Option<String>,
    pub faults: Option<Vec<String>>,
}


//FIXME: splitting the namespace is the naive way, we should keep the namespace
// and check for collisions instead
fn split_namespace(s: &str) -> &str {
    match s.find(':') {
        None => s,
        Some(index) => &s[index+1..],
    }
}

pub fn parse(bytes: &[u8]) -> Result<Wsdl, WsdlError> {
    let mut types = HashMap::new();
    let mut messages = HashMap::new();
    let mut operations = HashMap::new();

    let elements = Element::parse(bytes)?;
    println!("elements: {:#?}", elements);
    println!("line: {}", line!());

    let types_el = elements.get_child("types").ok_or(WsdlError::ElementNotFound("types"))?
        .children.iter().filter_map(|c| c.as_element())
        .next().ok_or(WsdlError::Empty)?;
    //println!("types: {:#?}", types_el);
    println!("line: {}", line!());

    for elem in types_el.children.iter().filter_map(|c| c.as_element()) {
        println!("type: {:#?}", elem);
        let name = elem.attributes.get("name").ok_or(WsdlError::AttributeNotFound("name"))?;
        //println!("name: {:?}", name);
    println!("line: {}", line!());

        // sometimes we have <element name="TypeName"><complexType>...</complexType></element>,
        // sometimes we have <complexType name="TypeName">...</complexType>
        //let current_child = elem.children.get(0).ok_or(WsdlError::Empty)?
        //    .as_element().ok_or(WsdlError::NotAnElement)?;

        let child = if elem.name == "complexType" {
            elem
        } else {
            elem.children.get(0).ok_or(WsdlError::Empty)?
                .as_element().ok_or(WsdlError::NotAnElement)?
        };

        if child.name == "complexType" {
            println!("line: {}", line!());
            let mut fields = HashMap::new();
            for field in child.children.get(0).ok_or(WsdlError::Empty)?
                .as_element().ok_or(WsdlError::NotAnElement)?
                    .children.iter().filter_map(|c| c.as_element()) {
                        println!("line: {}", line!());
                        let field_name = field.attributes.get("name").ok_or(WsdlError::AttributeNotFound("name"))?;
                        let field_type = field.attributes.get("type").ok_or(WsdlError::AttributeNotFound("type"))?;
                        let nillable = match field.attributes.get("nillable").map(|s| s.as_str()) {
                            Some("true") => true,
                            Some("false") => false,
                            _ => false,
                        };

                        let min_occurs = match field.attributes.get("minOccurs").map(|s| s.as_str()) {
                            None => None,
                            Some("unbounded") => Some(Occurence::Unbounded),
                            Some(n) => Some(Occurence::Num(n.parse().expect("occurence should be a number"))),
                        };
                        let max_occurs = match field.attributes.get("maxOccurs").map(|s| s.as_str()) {
                            None => None,
                            Some("unbounded") => Some(Occurence::Unbounded),
                            Some(n) => Some(Occurence::Num(n.parse().expect("occurence should be a number"))),
                        };
                        //println!("field {:?} -> {:?}", field_name, field_type);
                        //
                        let mut type_attributes = TypeAttribute { nillable, min_occurs, max_occurs };

                        let simple_type = match split_namespace(field_type.as_str()) {
                            "boolean" => SimpleType::Boolean,
                            "string" => SimpleType::String,
                            "int" => SimpleType::Int,
                            "float" => SimpleType::Float,
                            "dateTime" => SimpleType::DateTime,
                            s => SimpleType::Complex(s.to_string()),
                        };
                        fields.insert(field_name.to_string(), (type_attributes, simple_type));
                    }

            types.insert(name.to_string(), Type::Complex(ComplexType { fields }));
        } else {
            println!("child {:#?}", child);
            unimplemented!("not a complex type");
        }
    }

    println!("line: {}", line!());
    for message in elements
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|c| c.name == "message")
    {
        println!("line: {}", line!());
        println!("message: {:#?}", message);
        let name = message
            .attributes
            .get("name")
            .ok_or(WsdlError::AttributeNotFound("name"))?;
        let c = message.children.iter().filter_map(|c| c.as_element()).next().unwrap();
        println!("line: {}", line!());
        //FIXME: namespace
        let part_name = c
            .attributes
            .get("name")
            .ok_or(WsdlError::AttributeNotFound("name"))?
            .to_string();
        let part_element = split_namespace(c
            .attributes
            .get("element")
            .ok_or(WsdlError::AttributeNotFound("element"))?)
            .to_string();

        messages.insert(name.to_string(), Message { part_name, part_element });
    }

    println!("line: {}", line!());
    let port_type_el = elements.get_child("portType").ok_or(WsdlError::ElementNotFound("portType"))?;

    for operation in port_type_el.children.iter().filter_map(|c| c.as_element()) {
    println!("line: {}", line!());
        let operation_name = operation.attributes.get("name").ok_or(WsdlError::AttributeNotFound("name"))?;

        let mut input = None;
        let mut output = None;
        let mut faults = None;
        for child in operation.children.iter().filter_map(|c| c.as_element()) {
    println!("line: {}", line!());
            let message = split_namespace(child.attributes.get("message").ok_or(WsdlError::AttributeNotFound("message"))?);
            // FIXME: not testing for unicity
            match child.name.as_str() {
                "input" => input = Some(message.to_string()),
                "output" => output = Some(message.to_string()),
                "fault" => {
                    if faults.is_none() {
                        faults = Some(Vec::new());
                    }
                    faults.as_mut().map(|v| v.push(message.to_string()));
                },
                _ => return Err(WsdlError::ElementNotFound("operation member")),
            }
        }

        operations.insert(operation_name.to_string(), Operation {
            name: operation_name.to_string(),
            input,
            output,
            faults,
        });
    }

    println!("line: {}", line!());
    //FIXME: ignoring bindings for now
    //FIXME: ignoring service for now
    let service_name = elements.get_child("service").ok_or(WsdlError::ElementNotFound("service"))?
        .attributes.get("name").ok_or(WsdlError::AttributeNotFound("name"))?;
    println!("line: {}", line!());


    println!("service name: {}", service_name);
    println!("parsed types: {:#?}", types);
    println!("parsed messages: {:#?}", messages);
    println!("parsed operations: {:#?}", operations);

    Ok(Wsdl {
        name: service_name.to_string(),
        types,
        messages,
        operations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const WIKIPEDIA_WSDL: &[u8] = include_bytes!("../assets/wikipedia-example.wsdl");
    const EXAMPLE_WSDL: &[u8] = include_bytes!("../assets/example.wsdl");

    #[test]
    fn parse_example() {
        let res = parse(EXAMPLE_WSDL);
        println!("res: {:?}", res);
        res.unwrap();
        panic!();
    }
}
