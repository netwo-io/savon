use crate::wsdl::{parse, SimpleType, Type, Wsdl};
use case::CaseExt;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use std::{fs::File, io::Write};

pub trait ToElements {
    fn to_elements(&self) -> Vec<xmltree::Element>;
}

pub trait FromElement {
    fn from_element(element: &xmltree::Element) -> Result<Self, crate::Error>
        where Self: Sized;
}

impl<T: ToElements> ToElements for Option<T> {
    fn to_elements(&self) -> Vec<xmltree::Element> {
        match self {
            Some(e) => e.to_elements(),
            None => vec![],
        }
    }
}

/*impl<T: ToElements> for Vec<T> {
    fn to_elements(&self) -> Vec<xmltree::Element> {

        match self {
            Some(e) => e.to_elements(),
            None => vec![],
        }
    }
}*/

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
    let v = std::fs::read(path).unwrap();
    let mut output = File::create(out_path).unwrap();
    let wsdl = parse(&v[..]).unwrap();
    let generated = gen(&wsdl).unwrap();
    output.write_all(generated.as_bytes()).unwrap();
    output.flush().unwrap();

    Ok(())
}

pub fn gen(wsdl: &Wsdl) -> Result<String, GenError> {
    println!("wsdl:{:#?}", wsdl);
    let target_namespace = Literal::string(&wsdl.target_namespace);

    let operations = wsdl.operations.iter().map(|(name, operation)| {
        let op_name = Ident::new(&name.to_snake(), Span::call_site());
        let input_name = Ident::new(&operation.input.as_ref().unwrap().to_snake(), Span::call_site());
        let input_type = Ident::new(&operation.input.as_ref().unwrap().to_camel(), Span::call_site());

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
                let err_name = Ident::new(&format!("{}Error", name.to_camel()), Span::call_site());

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
                let type_name = Ident::new(&name.to_camel(), Span::call_site());

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
                            SimpleType::Complex(s) => Ident::new(&s.to_camel(), Span::call_site()),
                        };

                        let ft = match (attributes.min_occurs.as_ref(), attributes.max_occurs.as_ref()) {
                          (Some(_), Some(_)) => quote! { Vec<#ft> },
                          _ => quote! { #ft }
                        };
                        let ft = if attributes.nillable {
                            quote! { Option<#ft> }
                        } else {
                            ft
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

                        match (attributes.min_occurs.as_ref(), attributes.max_occurs.as_ref()) {
                          (Some(_), Some(_)) => if attributes.nillable {
                              quote! {
                                  self.#fname.as_ref().map(|v| v.iter().map(|i| {
                                      #prefix.with_children(i.to_elements())
                                  }).collect()).unwrap_or_else(Vec::new)
                              }
                          } else {
                              quote! {
                                  self.#fname.iter().map(|i| {
                                      #prefix.with_children(i.to_elements())
                                  }).collect()
                              }
                          },
                          _ => {
                              match field_type {
                                  SimpleType::Complex(_s) => quote!{ vec![#prefix.with_children(self.#fname.to_elements())]},
                                  _ => quote!{ vec![#prefix.with_text(self.#fname.to_string())] },
                              }

                          }
                        }
                    })
                    .collect::<Vec<_>>();

                let serialize_impl = if fields_serialize_impl.is_empty() {
                    quote! {
                        impl savon::gen::ToElements for #type_name {
                            fn to_elements(&self) -> Vec<xmltree::Element> {
                                vec![]
                            }
                        }
                    }

                }else {
                    quote! {
                        impl savon::gen::ToElements for #type_name {
                            fn to_elements(&self) -> Vec<xmltree::Element> {
                                vec![#(#fields_serialize_impl),*].drain(..).flatten().collect()
                            }
                        }
                    }
                };

                let fields_deserialize_impl = c
                    .fields
                    .iter()
                    .map(|(field_name, (attributes, field_type))| {
                        let fname = Ident::new(&field_name.to_snake(), Span::call_site());
                        let ftype = Literal::string(field_name);

                        let prefix = quote!{ #fname: element.get_at_path(&[#ftype]) };

                        match field_type {
                            SimpleType::Boolean => {
                                let ft = quote!{ #prefix.and_then(|e| e.as_boolean()) };
                                if attributes.nillable {
                                    quote!{ #ft.ok(),}
                                } else {
                                    quote!{ #ft?,}
                                }
                            },
                            SimpleType::String => {
                                let ft = quote!{ #prefix.and_then(|e| e.get_text().map(|s| s.to_string())
                                                     .ok_or(savon::rpser::xml::Error::Empty)
                                                     ) };
                                if attributes.nillable {
                                    quote!{ #ft.ok(),}
                                } else {
                                    quote!{ #ft?,}
                                }
                            },
                            SimpleType::Float => {
                                let ft = quote!{ #prefix.map_err(savon::Error::from).and_then(|e| e.get_text()
                                                     .ok_or(savon::rpser::xml::Error::Empty)
                                                     .map_err(savon::Error::from)
                                                     .and_then(|s| s.parse().map_err(savon::Error::from))) };
                                if attributes.nillable {
                                    quote!{ #ft.ok(),}
                                } else {
                                    quote!{ #ft?,}
                                }
                            },
                            SimpleType::Int => {
                                let ft = quote!{ #prefix.and_then(|e| e.as_long()) };
                                if attributes.nillable {
                                    quote!{ #ft.ok(),}
                                } else {
                                    quote!{ #ft?,}
                                }
                            },
                            SimpleType::DateTime => {
                                let ft = quote!{
                                    #prefix.and_then(|e| e.get_text()
                                                     .ok_or(savon::rpser::xml::Error::Empty)
                                                     ).map_err(savon::Error::from)
                                    .and_then(|s|
                                              s.parse::<savon::internal::chrono::DateTime<savon::internal::chrono::offset::Utc>>().map_err(savon::Error::from))
                                };
                                if attributes.nillable {
                                    quote!{ #ft.ok(),}
                                } else {
                                    quote!{ #ft?,}
                                }
                            },
                            SimpleType::Complex(s) => {
                                let complex_type = Ident::new(&s.to_camel(), Span::call_site());

                                match (attributes.min_occurs.as_ref(), attributes.max_occurs.as_ref()) {
                                    (Some(_), Some(_)) => {
                                        let ft = quote! {
                                            {
                                                let mut v = vec![];
                                                for elem in element.children.iter()
                                                    .filter_map(|c| c.as_element()) {
                                                        v.push(#complex_type::from_element(&elem)?);
                                                    }
                                                v
                                            },
                                        };

                                        if attributes.nillable {
                                            quote!{ #fname: Some(#ft) }
                                        } else {
                                            quote!{ #fname: #ft }
                                        }
                                    },
                                    _ => {
                                        let ft = quote!{ #prefix.map_err(savon::Error::from).and_then(|e| #complex_type::from_element(&e).map_err(savon::Error::from)) };
                                        if attributes.nillable {
                                            quote!{ #ft.ok(),}
                                        } else {
                                            quote!{ #ft?,}
                                        }
                                    }
                                }
                            },
                        }
                    })
                    .collect::<Vec<_>>();

                    let deserialize_impl = if fields_deserialize_impl.is_empty() {
                        quote! {
                            impl savon::gen::FromElement for #type_name {
                                fn from_element(_element: &xmltree::Element) -> Result<Self, savon::Error> {
                                    Ok(#type_name {
                                    })
                                }
                            }
                        }
                    } else {
                        quote! {
                            impl savon::gen::FromElement for #type_name {
                                fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
                                    Ok(#type_name {
                                        #(#fields_deserialize_impl)*
                                    })
                                }
                            }
                        }
                    };

                quote! {
                    #[derive(Clone, Debug, Default)]
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
                #[derive(Clone, Debug, Default)]
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
        use savon::internal::xmltree;
        use savon::rpser::xml::*;

        #(#types)*

        pub struct #service_name {
            pub base_url: String,
            pub client: savon::internal::reqwest::Client,
        }
        #(#messages)*

        #[allow(dead_code)]
        impl #service_name {
            pub fn new(base_url: String) -> Self {
                Self::with_client(base_url, savon::internal::reqwest::Client::new())
            }

            pub fn with_client(base_url: String, client: savon::internal::reqwest::Client) -> Self {
                #service_name {
                    base_url,
                    client,
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
                #[derive(Clone, Debug, Default)]
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
