use crate::wsdl::{parse, SimpleType, Type, Wsdl};
use case::CaseExt;
use proc_macro2::{Ident, Literal, Spacing, Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use std::{fs::File, io::Write};
use chrono::DateTime;

pub trait ToElements {
    fn to_elements(&self) -> Vec<xmltree::Element>;
}

pub trait FromElement {
    fn from_element(element: &xmltree::Element) -> Result<Self, crate::Error>
        where Self: Sized;
}

#[derive(Debug)]
pub enum GenError {
    Io(std::io::Error),
}

impl From<std::io::Error> for GenError {
    fn from(e: std::io::Error) -> Self {
        GenError::Io(e)
    }
}

pub fn gen_write(path: &str, out: &str) -> Result<(), ()> {
    let out_path = format!("{}/example.rs", out);
    let mut v = std::fs::read(path).unwrap();
    let mut output = File::create(out_path).unwrap();
    let wsdl = parse(&v[..]).unwrap();
    let generated = gen(&wsdl).unwrap();
    output.write_all(generated.as_bytes()).unwrap();
    output.flush();

    Ok(())
}

pub fn gen(wsdl: &Wsdl) -> Result<String, GenError> {
    println!("wsdl:{:#?}", wsdl);
    let target_namespace = Literal::string(&wsdl.target_namespace);

    let operations = wsdl.operations.iter().map(|(name, operation)| {
        let op_name = Ident::new(&name.to_snake(), Span::call_site());
        let input_name = Ident::new(&operation.input.as_ref().unwrap().to_snake(), Span::call_site());
        let input_type = Ident::new(operation.input.as_ref().unwrap(), Span::call_site());

        let op_str = Literal::string(&name);

        match (operation.output.as_ref(), operation.faults.as_ref()) {
            (None, None) => {
                quote! {
                    pub async fn #op_name(&self, #input_name: #input_type) -> Result<(), savon::Error> {
                        savon::http::one_way(&self.client, &self.base_url, #target_namespace, #op_str, &#input_name).await
                    }
                }
            },
            (None, Some(_)) => quote!{},
            (Some(out), None) => {
                let out_name = Ident::new(&out, Span::call_site());

                quote! {
                    pub async fn #op_name(&self, #input_name: #input_type) -> Result<Result<#out_name, ()>, savon::Error> {
                        savon::http::request_response(&self.client, &self.base_url, #target_namespace, #op_str, &#input_name).await
                    }
                }
            },
            (Some(out), Some(_)) => {
                let out_name = Ident::new(&out, Span::call_site());
                let err_name = Ident::new(&format!("{}Error", name), Span::call_site());

                quote! {
                    pub async fn #op_name(&self, #input_name: #input_type) -> Result<Result<#out_name, #err_name>, savon::Error> {
                        unimplemented!()
                        /*let req = hyper::http::request::Builder::new()
                            .method("POST")
                            .header("Content-Type", "text/xml-SOAP")
                            .header("MessageType", "Call")
                            .body(#input_name.as_xml())?;

                        let response: hyper::http::Response<String> = self.client.request(req).await?;
                        let body = response.body().await?;
                        if let Ok(out) = #out_name::from_xml(body) {
                            Ok(Ok(out))
                        } else {
                            Ok(#err_name::from_xml(body)?)
                        }
                        */
                    }
                }
            },
        }
    }).collect::<Vec<_>>();

    let types = wsdl
        .types
        .iter()
        .map(|(name, t)| {
            if let Type::Complex(c) = t {
                let type_name = Ident::new(&name, Span::call_site());

                let fields = c
                    .fields
                    .iter()
                    .map(|(field_name, (attributes, field_type))| {
                        let fname = Ident::new(&field_name.to_snake(), Span::call_site());
                        let ft = match field_type {
                            SimpleType::Boolean => Ident::new("bool", Span::call_site()),
                            SimpleType::String => Ident::new("String", Span::call_site()),
                            SimpleType::Float => Ident::new("f64", Span::call_site()),
                            SimpleType::Int => Ident::new("i64", Span::call_site()),
                            SimpleType::DateTime => Ident::new("chrono::DateTime", Span::call_site()),
                            SimpleType::Complex(s) => Ident::new(&s, Span::call_site()),
                        };

                        let ft = match (attributes.min_occurs.as_ref(), attributes.max_occurs.as_ref()) {
                          (Some(_), Some(_)) => quote! { Vec<#ft> },
                          _ => quote! { #ft }
                        };
                        let ft = match attributes.nillable {
                          true => quote! { Option<#ft> },
                          _ => ft
                        };

                        quote! {
                            pub #fname: #ft,
                        }
                    })
                    .collect::<Vec<_>>();

                let fields_serialize_impl = c
                    .fields
                    .iter()
                    .map(|(field_name, (attributes, field_type))| {
                        let fname = Ident::new(&field_name.to_snake(), Span::call_site());
                        //FIXME: handle more complex types
                        /*let ft = match field_type {
                            SimpleType::Boolean => Ident::new("bool", Span::call_site()),
                            SimpleType::String => Ident::new("String", Span::call_site()),
                            SimpleType::Float => Ident::new("f64", Span::call_site()),
                            SimpleType::Int => Ident::new("i64", Span::call_site()),
                            SimpleType::DateTime => Ident::new("String", Span::call_site()),
                            SimpleType::Complex(s) => Ident::new(&s, Span::call_site()),
                        };*/
                        let ftype = Literal::string(field_name);
                        let prefix = quote! { xmltree::Element::node(#ftype) };
                        let ser = match field_type {
                            SimpleType::Complex(s) => quote!{unimplemented!()},
                            _ => quote!{ self.#fname.to_string() },
                        };

                        quote! {
                            #prefix.with_text(#ser),
                        }
                    })
                    .collect::<Vec<_>>();

                let ns = Literal::string(&format!("ns:{}", name));
                let serialize_impl = quote! {
                    impl savon::gen::ToElements for #type_name {
                        fn to_elements(&self) -> Vec<xmltree::Element> {
                            //xmltree::Element::node(#ns)
                             //   .with_children(
                                    vec![
                                    #(#fields_serialize_impl)*
                                ]
                                //)
                        }
                    }
                };

                let fields_deserialize_impl = c
                    .fields
                    .iter()
                    .map(|(field_name, (attributes, field_type))| {
                        let fname = Ident::new(&field_name.to_snake(), Span::call_site());
                        let ftype = Literal::string(field_name);

                        let error = Literal::string(&format!("could not parse {}::{} as {:?}",
                                                             name, field_name, field_type));

                        let prefix = quote!{ #fname: element.get_at_path(&[#ftype]) };

                        let ft = match field_type {
                            SimpleType::Boolean => quote!{ #prefix.and_then(|e| e.as_boolean()) },
                            SimpleType::String => quote!{ #prefix.and_then(|e| e.as_string()) },
                            SimpleType::Float => quote!{ #prefix.and_then(|e| e.as_string().map_err(savon::Error::from).and_then(|s| s.parse().map_err(savon::Error::from))) },
                            SimpleType::Int => quote!{ #prefix.and_then(|e| e.as_long()) },
                            SimpleType::DateTime => quote!{
                                #fname: {
                                    #prefix.and_then(|e| e.as_string()).map_err(savon::Error::from)
                                      .and_then(|s| s.parse::<chrono::DateTime<chrono::offset::Utc>>().map_err(savon::Error::from))
                                }
                            },
                            SimpleType::Complex(s) => quote!{ #fname: {unimplemented!(#error); Ok(())} },
                        };

                        let ft = if attributes.nillable {
                          quote!{ #ft.ok(),}
                        } else {
                          quote!{ #ft?,}
                        };

                        ft
                    })
                    .collect::<Vec<_>>();

                let deserialize_impl = quote! {
                    impl savon::gen::FromElement for #type_name {
                        fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
                            Ok(#type_name {
                                #(#fields_deserialize_impl)*
                            })
                        }
                    }
                };

                quote! {
                    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
                    pub struct #type_name {
                        #(#fields)*
                    }

                    #serialize_impl

                    #deserialize_impl
                }
            } else {
                panic!();
            }
        })
        .collect::<Vec<_>>();

    let messages = wsdl
        .messages
        .iter()
        .map(|(message_name, message)| {
            let mname = Ident::new(&message_name, Span::call_site());
            let iname = Ident::new(&message.part_element, Span::call_site());

            quote! {
                #[derive(Clone, Debug, Default, Serialize, Deserialize)]
                pub struct #mname(pub #iname);

                impl savon::gen::ToElements for #mname {
                    fn to_elements(&self) -> Vec<xmltree::Element> {
                        self.0.to_elements()
                    }
                }

                impl savon::gen::FromElement for #mname {
                    fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
                        #iname::from_element(element).map(#mname)
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    let service_name = Ident::new(&wsdl.name, Span::call_site());

    let toks = quote! {
        use serde::{Deserialize, Serialize};
        use xmltree;
        use savon::rpser::xml::*;

        #(#types)*

        pub struct #service_name {
            pub base_url: String,
            pub client: reqwest::Client,
        }
        #(#messages)*

        impl #service_name {
            pub fn new(base_url: String) -> Self {
                #service_name {
                    base_url,
                    client: reqwest::Client::new(),
                }
            }

            #(#operations)*
        }
    };

    let operation_faults = wsdl
        .operations
        .iter()
        .filter(|(_, op)| op.faults.is_some())
        .map(|(name, operation)| {
            let op_error = Ident::new(&format!("{}Error", name), Span::call_site());

            let faults = operation
                .faults
                .as_ref()
                .unwrap()
                .iter()
                .map(|fault| {
                    let fault_name = Ident::new(&fault, Span::call_site());

                    quote! {
                          #fault_name(#fault_name),
                    }
                })
                .collect::<Vec<_>>();

            quote! {
                #[derive(Clone, Debug, Default, Serialize, Deserialize)]
                pub enum #op_error {
                    #(#faults)*
                }
            }
        })
        .collect::<Vec<_>>();

    let mut stream: TokenStream = toks;
    stream.extend(operation_faults);

    Ok(stream.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    const WIKIPEDIA_WSDL: &[u8] = include_bytes!("../assets/wikipedia-example.wsdl");
    const EXAMPLE_WSDL: &[u8] = include_bytes!("../assets/example.wsdl");
    use crate::wsdl::*;

    #[test]
    fn example() {
        let wsdl = parse(EXAMPLE_WSDL).unwrap();
        println!("wsdl: {:?}", wsdl);

        let res = gen(&wsdl).unwrap();

        println!("generated:\n{}", res);
        panic!();
    }
}
