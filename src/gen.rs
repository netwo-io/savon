use crate::wsdl::{parse, SimpleType, Type, Wsdl};
use case::CaseExt;
use proc_macro2::{Ident, Literal, Spacing, Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use std::{fs::File, io::Write};

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
    let operations = wsdl.operations.iter().map(|(name, operation)| {
        let op_name = Ident::new(name, Span::call_site());
        let input_name = Ident::new(&operation.input.as_ref().unwrap().to_snake(), Span::call_site());
        let input_type = Ident::new(operation.input.as_ref().unwrap(), Span::call_site());

        match (operation.output.as_ref(), operation.faults.as_ref()) {
            (None, None) => {
                quote! {
                    async fn #op_name(&self, #input_name: #input_type) -> Result<(), savon::Error> {
                        let req = hyper::http::request::Builder::new()
                            .method("POST")
                            .header("Content-Type", "text/xml-SOAP")
                            .header("MessageType", "Call")
                            .body(#input_name.as_xml())?;

                        let response: hyper::http::Response<String> = self.client.request(req).await?;
                        //let body = response.body().await?;
                    }
                }
            },
            (None, Some(_)) => quote!{},
            (Some(out), None) => {
                let out_name = Ident::new(&out, Span::call_site());

                quote! {
                    async fn #op_name(&self, #input_name: #input_type) -> Result<#out_name, savon::Error> {
                        let req = hyper::http::request::Builder::new()
                            .method("POST")
                            .header("Content-Type", "text/xml-SOAP")
                            .header("MessageType", "Call")
                            .body(#input_name.as_xml())?;

                        let response: hyper::http::Response<String> = self.client.request(req).await?;
                        let body = response.body().await?;
                        Ok(#out_name::from_xml(body)?)
                    }
                }
            },
            (Some(out), Some(_)) => {
                let out_name = Ident::new(&out, Span::call_site());
                let err_name = Ident::new(&format!("{}Error", name), Span::call_site());

                quote! {
                    async fn #op_name(&self, #input_name: #input_type) -> Result<Result<#out_name, #err_name>, savon::Error> {
                        let req = hyper::http::request::Builder::new()
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
                        let fname = Ident::new(&field_name, Span::call_site());
                        let ft = match field_type {
                            SimpleType::Boolean => Ident::new("bool", Span::call_site()),
                            SimpleType::String => Ident::new("String", Span::call_site()),
                            SimpleType::Float => Ident::new("f64", Span::call_site()),
                            SimpleType::Int => Ident::new("i64", Span::call_site()),
                            SimpleType::DateTime => Ident::new("String", Span::call_site()),
                            SimpleType::Complex(s) => Ident::new(&s, Span::call_site()),
                        };
                        quote! {
                            pub #fname: #ft,
                        }
                    })
                    .collect::<Vec<_>>();

                quote! {
                    #[derive(Clone, Debug, Default)]
                    pub struct #type_name {
                        #(#fields)*
                    }
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
                #[derive(Clone, Debug, Default)]
                pub struct #mname(pub #iname);
            }
        }).collect::<Vec<_>>();

    let service_name = Ident::new(&wsdl.name, Span::call_site());

    let toks = quote! {
        #(#types)*

        struct #service_name {
            base_url: String,
            client: hyper::client::Client,
        }
        #(#messages)*

        impl #service_name {
            pub fn new(base_url: String) -> Self {
                #service_name {
                    base_url,
                    client: hyper::client::Client::new(),
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
                enum #op_error {
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
