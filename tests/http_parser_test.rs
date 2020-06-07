#![allow(non_snake_case)]

use std::fs;
use vial::{
    http_parser::{parse, Status},
    Request,
};

////
// helpers

fn fixture(name: &str) -> String {
    fs::read_to_string(name).unwrap()
}

fn parse_fixture(name: &str) -> Request {
    match parse(fixture(name).as_bytes().to_vec()).unwrap() {
        Status::Complete(request) => request,
        _ => panic!("Expected Status::Complete"),
    }
}

////
// tests

#[test]
fn parses_simple_GET() {
    let request = parse_fixture("tests/http/simple_GET.txt");
    assert_eq!("/", request.path());
    assert_eq!("GET", request.method());
    assert_eq!("www.codecademy.com", request.header("Host").unwrap());
}

#[test]
fn parses_another_GET() {
    let request = parse_fixture("tests/http/another_GET.txt");
    assert_eq!("/docs/index.html", request.path());
    assert_eq!("GET", request.method());
    assert_eq!("www.nowhere123.com", request.header("Host").unwrap());
    assert_eq!("en-us", request.header("Accept-Language").unwrap());
    assert_eq!(
        "Mozilla/4.0 (compatible; MSIE 6.0; Windows NT 5.1)",
        request.header("User-Agent").unwrap()
    );
    assert_eq!(
        "image/gif, image/jpeg, */*",
        request.header("Accept").unwrap()
    );
}

#[test]
fn parses_big_GET() {
    let request = parse_fixture("tests/http/big_GET.txt");
    assert_eq!("/big", request.path());
    assert_eq!("GET", request.method());
    assert!(request.header("X-SOME-HEADER").is_some());
    assert!(request.header("X-SOMEOTHER-HEADER").is_some());
    assert_eq!(request.header("X-ONEMORE-HEADER").unwrap(),
        "gWbWykBHgObDHriErqIKRBqebBekBpHsqUJqQcDtDctkaeeFBwNelgvzigaEkUPKAfcnYGhgbzDOvGumdewDzCqOantKfsvaZuggZaTjqtUzOXHVYwsSjknsMTPyWzvzGrNdRExaSIjiehYvuSAMdOMpwakKlKxCPwYAyAlpqXpoiargAZnAVIRfUJVpBnotmQRsDtAZoFfSXyRvqGQluzWWVTOCItNSCqBPUfFQGoxoSewvuSStgDtCYfCnFCFNczEwGkLiPidmrpbQDPuIvopUbxvojuUrBfgjoTwslrnDIJGAWIMoMkOQzYdzxVaCDfSQlmHwkpdkxByhuWXmuLgAzgJvIuhAMMlXaHIMcGmymGCxsgUjUkzKwrzafCsfkSivOXIzNSmTGhdgBufQTqdlRbuDBZijZCOXmpwhKFzlaSleXzgMaEpDiEjxzPUwIOwhomPDVSzaTqEZCpivNWyfunffMNUaLdkxLudYEpSgwTOGUipJjvXbocrKbfFG"
    );
}

#[test]
fn parses_simple_POST() {
    let fixture = fs::File::open("tests/http/simple_POST.txt").unwrap();
    let request = Request::from_reader(fixture).unwrap();
    assert_eq!("/cgi-bin/process.cgi", request.path());
    assert_eq!("POST", request.method());
    assert_eq!(Some("hi there"), request.form("content"));
    assert_eq!(Some("1234"), request.form("licenseID"));
    assert_eq!(Some("<abc></abc>"), request.form("paramsXML"));
    assert_eq!(None, request.form("something"));
}

#[test]
fn rejects_malformed_headers() {}

#[test]
fn rejects_expected_but_no_body() {}
