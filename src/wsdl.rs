//! WSDL inspection helpers.

use http;
use std::collections::HashMap;
use xml::reader::{EventReader, XmlEvent};

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

/// Fetch WSDL from specified URL and store results in `Wsdl` structure.
pub fn fetch(url: &str) -> http::Result<Wsdl> {
    let response = try!(http::get(&url));
    let mut bytes = response.body.as_bytes();

    let mut operations = HashMap::new();

    let parser = EventReader::new(&mut bytes);
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                ref name,
                ref attributes,
                ref namespace,
            }) => {
                if name.to_string().contains("wsdl:operation") {
                    if let (Some(name_attribute), Some(impl_url)) = (
                        attributes.iter().find(|a| a.name.to_string() == "name"),
                        namespace.get("impl"),
                    ) {
                        operations.insert(
                            name_attribute.value.to_string(),
                            Operation {
                                url: impl_url.into(),
                            },
                        );
                    }
                }
            }
            Err(e) => {
                error!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(Wsdl { operations })
}
