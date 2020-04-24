#[derive(Debug)]
pub enum Error {
  Wsdl(crate::wsdl::WsdlError),
  Serde(serde_xml_rs::Error),
  Reqwest(reqwest::Error),
}

impl From<crate::wsdl::WsdlError> for Error {
  fn from(e: crate::wsdl::WsdlError) -> Self {
      Error::Wsdl(e)
  }
}

impl From<serde_xml_rs::Error> for Error {
  fn from(e: serde_xml_rs::Error) -> Self {
      Error::Serde(e)
  }
}

impl From<reqwest::Error> for Error {
  fn from(e: reqwest::Error) -> Self {
      Error::Reqwest(e)
  }
}
