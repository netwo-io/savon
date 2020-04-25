# savon, a SOAP client generator for Rust

savon generates code from a WSDL file, that you can then include
in your project. It will generate serialization and deserialization
code, along with an async HTTP client API (based on reqwest).

## Usage

in `Cargo.toml`:

```toml
[dependencies]
savon = "0.1"
```

in `build.rs`:

```rust
fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let s = savon::gen::gen_write("./assets/example.wsdl", &out_dir).unwrap();
}
```

Finally, in your code:

```rust
mod soap {
    include!(concat!(env!("OUT_DIR"), "/example.rs"));
}
```

You can then use it as follows:

```rust
    let client = soap::StockQuoteService::new("http://example.com".to_string());
    let res = client.get_last_trade_price(soap::GetLastTradePriceInput(TradePriceRequest { ticker_symbol: "SOAP".to_string() })).await?;
```

## Under the hood

If you use the following WSDL file as input:

```wsdl
<?xml version="1.0"?>
<definitions name="StockQuote"
             targetNamespace="http://example.com/stockquote.wsdl"
             xmlns:tns="http://example.com/stockquote.wsdl"
             xmlns:xsd1="http://example.com/stockquote.xsd"
             xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
             xmlns="http://schemas.xmlsoap.org/wsdl/">

  <types>
    <schema targetNamespace="http://example.com/stockquote.xsd"
            xmlns="http://www.w3.org/2000/10/XMLSchema">
      <element name="TradePriceRequest">
        <complexType>
          <all>
            <element name="tickerSymbol" type="string"/>
          </all>
        </complexType>
      </element>
      <element name="TradePrice">
         <complexType>
           <all>
             <element name="price" type="float"/>
           </all>
         </complexType>
      </element>
    </schema>
  </types>

  <message name="GetLastTradePriceInput">
    <part name="body" element="xsd1:TradePriceRequest"/>
  </message>

  <message name="GetLastTradePriceOutput">
    <part name="body" element="xsd1:TradePrice"/>
  </message>

  <portType name="StockQuotePortType">
    <operation name="GetLastTradePrice">
      <input message="tns:GetLastTradePriceInput"/>
      <output message="tns:GetLastTradePriceOutput"/>
    </operation>
  </portType>

  <binding name="StockQuoteSoapBinding" type="tns:StockQuotePortType">
    <soap:binding style="document" transport="http://schemas.xmlsoap.org/soap/http"/>
    <operation name="GetLastTradePrice">
      <soap:operation soapAction="http://example.com/GetLastTradePrice"/>
      <input>
        <soap:body use="literal"/>
      </input>
      <output>
        <soap:body use="literal"/>
      </output>
    </operation>
  </binding>

  <service name="StockQuoteService">
    <documentation>My first service</documentation>
    <port name="StockQuotePort" binding="tns:StockQuoteSoapBinding">
      <soap:address location="http://example.com/stockquote"/>
    </port>
  </service>

</definitions>
```

It will generate this code:

```rust
use savon::internal::xmltree;
use savon::rpser::xml::*;

#[derive(Clone, Debug, Default)]
pub struct TradePriceRequest {
    pub ticker_symbol: String,
}

impl savon::gen::ToElements for TradePriceRequest {
    fn to_elements(&self) -> Vec<xmltree::Element> {
        vec![vec![
            xmltree::Element::node("tickerSymbol").with_text(self.ticker_symbol.to_string())
        ]]
        .drain(..)
        .flatten()
        .collect()
    }
}

impl savon::gen::FromElement for TradePriceRequest {
    fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
        Ok(TradePriceRequest {
            ticker_symbol: element.get_at_path(&["tickerSymbol"]).and_then(|e| {
                e.get_text()
                    .map(|s| s.to_string())
                    .ok_or(savon::rpser::xml::Error::Empty)
            })?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct TradePrice {
    pub price: f64,
}

impl savon::gen::ToElements for TradePrice {
    fn to_elements(&self) -> Vec<xmltree::Element> {
        vec![vec![
            xmltree::Element::node("price").with_text(self.price.to_string())
        ]]
        .drain(..)
        .flatten()
        .collect()
    }
}

impl savon::gen::FromElement for TradePrice {
    fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
        Ok(TradePrice {
            price: element
                .get_at_path(&["price"])
                .map_err(savon::Error::from)
                .and_then(|e| {
                    e.get_text()
                        .ok_or(savon::rpser::xml::Error::Empty)
                        .map_err(savon::Error::from)
                        .and_then(|s| s.parse().map_err(savon::Error::from))
                })?,
        })
    }
}

pub struct StockQuoteService {
    pub base_url: String,
    pub client: savon::internal::reqwest::Client,
}

#[derive(Clone, Debug, Default)]
pub struct GetLastTradePriceOutput(pub TradePrice);

impl savon::gen::ToElements for GetLastTradePriceOutput {
    fn to_elements(&self) -> Vec<xmltree::Element> {
        self.0.to_elements()
    }
}

impl savon::gen::FromElement for GetLastTradePriceOutput {
    fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
        TradePrice::from_element(element).map(GetLastTradePriceOutput)
    }
}

#[derive(Clone, Debug, Default)]
pub struct GetLastTradePriceInput(pub TradePriceRequest);

impl savon::gen::ToElements for GetLastTradePriceInput {
    fn to_elements(&self) -> Vec<xmltree::Element> {
        self.0.to_elements()
    }
}

impl savon::gen::FromElement for GetLastTradePriceInput {
    fn from_element(element: &xmltree::Element) -> Result<Self, savon::Error> {
        TradePriceRequest::from_element(element).map(GetLastTradePriceInput)
    }
}

#[allow(dead_code)]
impl StockQuoteService {
    pub fn new(base_url: String) -> Self {
        Self::with_client(base_url, savon::internal::reqwest::Client::new())
    }

    pub fn with_client(base_url: String, client: savon::internal::reqwest::Client) -> Self {
        StockQuoteService { base_url, client }
    }

    pub async fn get_last_trade_price(
        &self,
        get_last_trade_price_input: GetLastTradePriceInput,
    ) -> Result<Result<GetLastTradePriceOutput, ()>, savon::Error> {
        savon::http::request_response(
            &self.client,
            &self.base_url,
            "http://example.com/stockquote.wsdl",
            "GetLastTradePrice",
            &get_last_trade_price_input,
        )
        .await
    }
}
```
