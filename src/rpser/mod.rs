//! Remote procedule call implementation and serialization to XML.

pub mod xml;

use std::fmt;
use std::result;

use self::xml::BuildElement;
use xmltree::Element;

/// XML method representation.
#[derive(Debug)]
pub struct Method {
    pub name: String,
    pub args: Vec<Element>,
}

impl Method {
    /// Create new method with name.
    pub fn new(name: &str) -> Method {
        Method {
            name: name.into(),
            args: vec![],
        }
    }

    /// Add argument to method.
    ///
    /// The `arg` is XML Element.
    pub fn with(mut self, arg: Element) -> Self {
        self.args.push(arg);
        self
    }

    /// Convert method to full XML envelope.
    pub fn as_xml(&self, api_url: &str) -> String {
        let namespace = "api";

        let envelope = Element::node("soap:Envelope")
            .with_attr("xmlns:soap", "http://schemas.xmlsoap.org/soap/envelope/")
            .with_attr(format!("xmlns:{}", namespace), api_url)
            .with_children(vec![
                Element::node("soap:Header"),
                Element::node("soap:Body").with_child(
                    Element::node(format!("{}:{}", namespace, self.name))
                        .with_children_from_iter(self.args.iter()),
                ),
            ]);

        envelope.to_string()
    }
}

/// XML response representation.
#[derive(Debug)]
pub struct Response {
    pub body: Element,
}

impl Response {
    /// Parse response from XML.
    pub fn from_xml(xml: &str) -> Result<Response> {
        let mut bytes = xml.as_bytes();
        let mut element = Element::parse(&mut bytes).unwrap();

        if element.name != "Envelope" {
            return Err(RpcError::UnexpectedElement { tag: element.name });
        }
        element = try!(element.descend(&["Body"]));
        element = try!(element.descend_first());

        if element.name == "Fault" {
            return Err(RpcError::Fault {
                fault_code: try!(element.get_at_path(&["faultcode"]))
                    .text
                    .unwrap_or_default(),
                fault_string: try!(element.get_at_path(&["faultstring"]))
                    .text
                    .unwrap_or_default(),
                fault_detail: Box::new(try!(element.get_at_path(&["detail"]))),
            });
        }

        Ok(Response { body: element })
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {:?}", self.name, self.args)
    }
}

/// Method parsing / response error.
#[derive(Debug, PartialEq)]
pub enum RpcError {
    Fault {
        fault_code: String,
        fault_string: String,
        fault_detail: Box<Element>,
    },
    XmlError {
        error: self::xml::Error,
    },
    ExpectedElementText {
        tag: String,
    },
    UnexpectedElement {
        tag: String,
    },
    ElementWasEmpty {
        name: String,
    },
    ElementNotFound {
        path: Vec<String>,
    },
}

impl From<self::xml::Error> for RpcError {
    fn from(other: self::xml::Error) -> RpcError {
        RpcError::XmlError { error: other }
    }
}

pub type Result<T> = result::Result<T, RpcError>;

#[cfg(test)]
mod test {
    use super::*;
    use rpser::xml::BuildElement;

    #[test]
    fn can_deal_with_fault() {
        let faulty_response = r#"<?xml version="1.0" encoding="utf-8"?>
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
                <soapenv:Body>
                    <soapenv:Fault>
                        <faultcode>soapenv:Server.userException</faultcode>
                        <faultstring>com.atlassian.confluence.rpc.AuthenticationFailedException: Attempt to log in user 'ADUser' failed - incorrect username/password combination.</faultstring>
                        <detail>
                            <com.atlassian.confluence.rpc.AuthenticationFailedException xsi:type="ns1:AuthenticationFailedException" xmlns:ns1="http://rpc.confluence.atlassian.com"/>
                            <ns2:hostname xmlns:ns2="http://xml.apache.org/axis/">jira</ns2:hostname>
                        </detail>
                    </soapenv:Fault>
                </soapenv:Body>
            </soapenv:Envelope>
        "#;

        match Response::from_xml(faulty_response) {
            Err(RpcError::Fault {
                fault_code,
                fault_string,
                ..
            }) => {
                assert_eq!(fault_code, "soapenv:Server.userException");
                assert_eq!(fault_string, "com.atlassian.confluence.rpc.AuthenticationFailedException: Attempt to log in user 'ADUser' failed - incorrect username/password combination.");
            }
            other => panic!(
                "expected to receive fault in this test, received {:?}",
                other
            ),
        };
    }

    #[test]
    fn returns_result_element() {
        let ok_response = r#"<?xml version="1.0" encoding="utf-8"?>
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
                <soapenv:Body>
                    <ns1:loginResponse soapenv:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/" xmlns:ns1="https://confluence/plugins/servlet/soap-axis1/confluenceservice-v2">
                        <loginReturn xsi:type="xsd:string">a3a8ecc6d5</loginReturn>
                    </ns1:loginResponse>
                </soapenv:Body>
            </soapenv:Envelope>
        "#;

        match Response::from_xml(ok_response) {
            Ok(response) => {
                assert_eq!(response.body.name, "loginResponse");
                let return_element = response.body.descend_first().unwrap();
                assert_eq!(return_element.name, "loginReturn");
                assert_eq!(return_element.text, Some("a3a8ecc6d5".into()));
            }
            other => panic!(
                "expected to receive fault in this test, received {:?}",
                other
            ),
        };
    }
}
