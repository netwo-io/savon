#[macro_use]
extern crate log;

mod soap {
    include!(concat!(env!("OUT_DIR"), "/example.rs"));
}

#[tokio::main]
async fn main() -> Result<(), savon::Error> {
    pretty_env_logger::init();

    let base_url ="http://webservices.oorsprong.org/websamples.countryinfo/CountryInfoService.wso";
    info!("Hello, world!");

    let client = soap::CountryInfoService::new(base_url.to_string());

    let res = client.list_of_continents_by_name(soap::ListOfContinentsByNameSoapRequest(soap::ListOfContinentsByName{})).await?;

    info!("res: {:?}", res);

    Ok(())
}
