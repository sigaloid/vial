#[cfg(feature = "compression")]
use fly_accept_encoding::Encoding;
use std::net::{TcpStream, SocketAddr, IpAddr, Ipv4Addr};
use {
    crate::{http_parser, util, Error, Result, TypeCache},
    std::{borrow::Cow, collections::HashMap, fmt, io, rc::Rc, str},
};

#[cfg(feature = "cookies")]
use basic_cookies::Cookie;

/// A `(start, end)` tuple representing a the location of some part of
/// a Request in a raw buffer, such as the requested URL's path.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Span(pub usize, pub usize);

impl Span {
    /// Create a new, empty Span.
    pub fn new() -> Self {
        Self::default()
    }

    /// Is this span empty?
    pub fn is_empty(&self) -> bool {
        self.0 == self.1
    }

    /// Find and return the str this span represents from the given
    /// buffer, which should be the raw HTTP request.
    pub fn from_buf<'buf>(&self, buf: &'buf [u8]) -> &'buf str {
        if self.is_empty() {
            ""
        } else if self.1 >= self.0 && self.1 <= buf.len() {
            str::from_utf8(&buf[self.0..self.1]).unwrap_or("?")
        } else {
            "?"
        }
    }
}

/// Contains information about a single request.
pub struct Request {
    /// Remote address.
    remote_addr: SocketAddr,

    /// The raw request.
    buffer: Vec<u8>,

    /// Includes `?query` and starts with `/`.
    /// Calling `request.path()` delivers just the path without any
    /// `?query` - use `request.full_path()` to get the full story.
    path: Span,

    /// HTTP Method
    method: Span,

    /// Sent Headers
    headers: Vec<(Span, Span)>,

    /// Request Body (POST)
    body: Span,

    /// Maps of form and URL args, percent decoded.
    args: HashMap<String, String>,
    form: HashMap<String, String>,

    /// Local request cache.
    cache: Rc<TypeCache>,

    #[cfg(feature = "cookies")]
    cookies: Vec<(String, String)>,
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut headers = HashMap::new();
        for (k, v) in &self.headers {
            headers.insert(k.from_buf(&self.buffer), v.from_buf(&self.buffer));
        }
        f.debug_struct("Request")
            .field("method", &self.method.from_buf(&self.buffer))
            .field("path", &self.path.from_buf(&self.buffer))
            .field("full_path", &self.path.from_buf(&self.buffer))
            .field("headers", &headers)
            .finish()
    }
}

impl Request {
    /// Create a new Request from a raw one. You probably want
    /// `default()` to get an empty `Request`.
    #[must_use]
    pub fn new(
        method: Span,
        path: Span,
        headers: Vec<(Span, Span)>,
        body: Span,
        buffer: Vec<u8>,
    ) -> Self {
        Self {
            buffer,
            path,
            method,
            headers,
            body,
            ..Self::default()
        }
    }
    /// Produce an empty Request.
    #[must_use]
    pub fn default() -> Self {
        Self {
            remote_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
            path: Span::new(),
            method: Span::new(),
            body: Span::new(),
            headers: Vec::new(),
            args: HashMap::new(),
            form: HashMap::new(),
            buffer: Vec::new(),
            cache: Rc::new(TypeCache::new()),

            #[cfg(feature = "cookies")]
            cookies: vec![],
        }
    }

    /// Read a raw HTTP request from `reader` and create an
    /// appropriate `Request` to represent it.
    ///
    /// # Errors
    /// This function will return an error if the request's reader blocks, or if the request cannot be parsed.
    pub fn from_reader<R: io::Read>(mut reader: R) -> Result<Self> {
        let mut buffer = Vec::with_capacity(512);
        loop {
            match reader.read_to_end(&mut buffer) {
                Ok(_) => break,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if !buffer.is_empty() {
                        break;
                    }
                }
                Err(e) => return Err(Error::IO(e)),
            };
        }

        let mut req = http_parser::parse(std::mem::take(&mut buffer))?;

        if req.header("Content-Length").is_some() {
            req.parse_form();
        }

        #[cfg(feature = "cookies")]
        {
            if let Some(cookie) = req.header("Cookie") {
                if let Ok(cookies) = Cookie::parse(&cookie.into_owned()) {
                    for cookie in cookies {
                        req.cookies.push((
                            cookie.get_name().to_string(),
                            cookie.get_value().to_string(),
                        ));
                    }
                }
            }
        }

        Ok(req)
    }

    /// Read a raw HTTP request from `TcpStream` and create an
    /// appropriate `Request` to represent it.
    /// # Errors
    /// This function will error if the stream cannot be set to non-blocking (and downstream, if the request's reader blocks, or if the request cannot be parsed)
    pub fn from_stream(stream: &TcpStream) -> Result<Self> {
        stream.set_nonblocking(true)?;
        Self::from_reader(stream)
    }

    /// Sets the remote address of the request.
    pub fn set_remote_addr(&mut self, socket_addr: SocketAddr) {
        self.remote_addr = socket_addr;
    }

    /// Remote address of the request.
    #[must_use]
    pub fn remote_addr(&self) -> &SocketAddr {
        &self.remote_addr
    }

    /// Path requested, starting with `/` and not including `?query`.
    #[must_use]
    pub fn path(&self) -> &str {
        let span = self
            .full_path()
            .find('?')
            .map_or(self.path, |idx| Span(self.path.0, self.path.0 + idx));
        span.from_buf(&self.buffer)
    }

    /// Full path requested, starting with `/` and including `?query`.
    #[must_use]
    pub fn full_path(&self) -> &str {
        self.path.from_buf(&self.buffer)
    }

    /// Create a request from an arbitrary path. Used in testing.
    #[must_use]
    pub fn from_path(path: &str) -> Self {
        Self::default().with_path(path)
    }

    /// Give a request an arbitrary `path`. Can be used in tests or
    /// with `filter`.
    pub fn set_path(&mut self, path: &str) {
        self.path = Span(self.buffer.len(), self.buffer.len() + path.len());
        self.buffer.extend(path.as_bytes());
    }

    /// Give a request an arbitrary `path`. Can be used in tests or
    /// with `filter`.
    #[must_use]
    pub fn with_path(mut self, path: &str) -> Self {
        self.set_path(path);
        self
    }

    /// Raw body of HTTP request. If you are using methods like
    /// `with_path` or `set_arg` this will not accurately represent
    /// the raw HTTP request that was made.
    #[must_use]
    pub fn body(&self) -> &str {
        self.body.from_buf(&self.buffer)
    }

    /// Give this Request an arbitrary body from a string.
    pub fn set_body<S: AsRef<str>>(&mut self, body: S) {
        self.body = Span(self.buffer.len(), self.buffer.len() + body.as_ref().len());
        self.buffer.extend(body.as_ref().as_bytes());
    }

    /// Give this Request an arbitrary body from a string and return
    /// the new Request.
    pub fn with_body<S: AsRef<str>>(mut self, body: S) -> Self {
        self.set_body(body);
        self
    }

    /// Body of HTTP request deserialized as a JSON value.
    ///
    /// The `json_serde` feature must be enabled in `Cargo.toml`.
    /// # Errors
    /// Errors if the javascript within the body is invalid.
    #[cfg(feature = "json_serde")]
    pub fn json<'a, T: serde::Deserialize<'a>>(&'a self) -> serde_json::Result<T> {
        serde_json::from_str(self.body())
    }

    /// HTTP Method
    #[must_use]
    pub fn method(&self) -> &str {
        self.method.from_buf(&self.buffer)
    }

    /// Give this Request a new HTTP Method.
    pub fn set_method(&mut self, method: &str) {
        self.method = Span(self.buffer.len(), self.buffer.len() + method.len());
        self.buffer.extend(method.as_bytes());
    }

    /// Give this Request a new HTTP Method and return the new Request.
    #[must_use]
    pub fn with_method(mut self, method: &str) -> Self {
        self.set_method(method);
        self
    }

    /// In a route defined with `routes!` like `"/names/:name"`,
    /// calling `request.arg("name")` will return `Some("peter")` when
    /// the request is `/names/peter`.
    #[must_use]
    pub fn arg(&self, name: &str) -> Option<&str> {
        self.args.get(name).map(std::convert::AsRef::as_ref)
    }

    /// Replace or set a new value for an arbitrary URL argument from
    /// a `filter` or in a test.
    pub fn set_arg(&mut self, name: String, value: String) {
        self.args.insert(name, value);
    }

    #[doc(hidden)]
    /// For testing. You should use [`header()`](#method.header) to
    /// get a specific header from this Request.
    #[must_use]
    pub fn headers(&self) -> &Vec<(Span, Span)> {
        &self.headers
    }

    /// Get a header value. `name` is case insensitive.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<Cow<'_, str>> {
        let name = name.to_lowercase();
        let headers = self
            .headers
            .iter()
            .filter(|(n, _)| n.from_buf(&self.buffer).to_ascii_lowercase() == name)
            .map(|(_, v)| v.from_buf(&self.buffer).trim_end());

        let count = headers.clone().count();
        if count == 0 {
            None
        } else {
            Some(Cow::from(headers.collect::<Vec<_>>().join(", ")))
        }
    }

    /// Was the given form value sent?
    pub fn has_form(&mut self, name: &str) -> bool {
        self.form(name).is_some()
    }

    /// Return a value from the `POSTed` form data.
    #[must_use]
    pub fn form(&self, name: &str) -> Option<&str> {
        self.form.get(name).map(std::convert::AsRef::as_ref)
    }

    /// Replace or set a new value for an arbitrary URL argument from
    /// a `filter` or in a test.
    pub fn set_form(&mut self, name: &str, value: &str) {
        self.form.insert(name.to_string(), value.to_string());
    }

    /// Parse and decode form POST data into a Hash. Should be called
    /// when this Request is created.
    #[doc(hidden)]
    pub fn parse_form(&mut self) {
        let mut map = HashMap::new();
        for kv in self.body().split('&') {
            let mut parts = kv.splitn(2, '=');
            if let Some(key) = parts.next() {
                if let Some(val) = parts.next() {
                    map.insert(key.to_string(), util::decode_form_value(val));
                } else {
                    map.insert(key.to_string(), String::new());
                };
            }
        }
        self.form = map;
    }

    /// Was the given query value sent?
    #[must_use]
    pub fn has_query(&self, name: &str) -> bool {
        self.query(name).is_some()
    }

    /// Return a value from the ?querystring=
    #[must_use]
    pub fn query(&self, name: &str) -> Option<&str> {
        let idx = self.full_path().find('?')?;
        self.full_path()[idx + 1..].split('&').find_map(|s| {
            if s.starts_with(name) && s[name.len()..].starts_with('=') {
                Some(&s[name.len() + 1..])
            } else {
                None
            }
        })
    }
    /// Return the compression type from accept-encoding header (none)
    #[cfg(not(feature = "compression"))]
    #[must_use]
    pub fn compression(&self) -> Option<crate::Compression> {
        None
    }
    /// Return the compression type from accept-encoding header
    #[cfg(feature = "compression")]
    #[must_use]
    pub fn compression(&self) -> Option<crate::Compression> {
        use crate::Compression;
        if let Some(content_encoding) = self.header("Accept-Encoding") {
            if let Ok(header_value) = http::header::HeaderValue::from_str(&content_encoding) {
                let mut headers = http::header::HeaderMap::new();
                headers.insert(http::header::ACCEPT_ENCODING, header_value);
                if let Ok(Some(compression)) = fly_accept_encoding::parse(&headers) {
                    return match compression {
                        Encoding::Gzip => Some(Compression::Gzip),
                        Encoding::Deflate => Some(Compression::Deflate),
                        Encoding::Brotli => Some(Compression::Brotli),
                        Encoding::Zstd => Some(Compression::Zstd),
                        Encoding::Identity => None,
                    };
                }
            }
        }
        None
    }
    /// Return the user-agent header
    #[must_use]
    pub fn user_agent(&self) -> Option<String> {
        self.header("User-Agent").map(|ua| ua.to_string())
    }
    /// Return the Do Not Track header
    #[must_use]
    pub fn do_not_track(&self) -> bool {
        match self.header("DNT") {
            Some(dnt) => dnt == "1",
            None => false,
        }
    }
    /// Return the Accept-Language, if exists
    #[must_use]
    pub fn accept_language(&self) -> Option<String> {
        self.header("Accept-Language").map(|al| al.to_string())
    }
    /// Return the content-type, if exists
    #[must_use]
    pub fn content_type(&self) -> Option<String> {
        self.header("Content-Type").map(|ct| ct.to_string())
    }
    /// Request's `cache()` lives for only a single Request, but can
    /// nonethenevertheless be useful to prevent looking up the same
    /// data over and over. The cache is based on the return type of
    /// the function or closure you pass to `cache()` using
    /// [`TypeCache`](struct.TypeCache.html), so make sure to create
    /// little wrapper structs if you want different functions to
    /// return the same common types, like `Vec<String>`:
    ///
    /// ```rust
    /// struct PageNames(Vec<String>);
    /// struct UserNames(Vec<String>);
    /// ```
    ///
    /// Here's an example:
    ///
    /// ```no_run
    /// use vial::prelude::*;
    /// # mod page { pub struct Page { pub name: String } }
    /// use page::Page;
    /// # mod db { use super::Page; pub fn lookup(query: &str) -> Vec<Page> { vec![] } }
    ///
    /// routes! {
    ///     GET "/" => list;
    /// }
    ///
    /// struct PageNames(Vec<String>);
    ///
    /// fn all_pages(_: &Request) -> Vec<Page> {
    ///     db::lookup("select * from pages")
    /// }
    ///
    /// fn page_names(req: &Request) -> PageNames {
    ///     PageNames(req.cache(all_pages)
    ///         .iter()
    ///         .map(|page| page.name.clone())
    ///         .collect::<Vec<_>>())
    /// }
    ///
    /// fn list_of_names(req: &Request) -> String {
    ///     req.cache(page_names)
    ///         .0
    ///         .iter()
    ///         .map(|name| format!("<li>{}</li>", name))
    ///         .collect::<Vec<_>>()
    ///         .join("\n")
    /// }
    ///
    /// fn list(req: Request) -> impl Responder {
    ///     format!(
    ///         "<html>
    ///             <head><title>{title}</title></head>
    ///             <body>
    ///                 <h1>{title}</h1>
    ///                 <h3>There are {page_count} pages:</h3>
    ///                 <ul>
    ///                     {pages}
    ///                 </ul>
    ///             </body>
    ///         </html>",
    ///         title = "List Pages",
    ///         page_count = req.cache(all_pages).len(),
    ///         pages = req.cache(list_of_names),
    ///     )
    /// }
    ///
    /// fn main() {
    ///     run!().unwrap();
    /// }
    /// ```
    pub fn cache<T, F>(&self, fun: F) -> &T
    where
        F: FnOnce(&Self) -> T,
        T: Send + Sync + 'static,
    {
        self.cache.get().unwrap_or_else(|| {
            self.cache.set(fun(self));
            self.cache
                .get()
                .expect("Failed to get cache value in above line!")
        })
    }

    /// Access to global shared state defined with the
    /// [`vial::use_state!`](macro.use_state.html) macro before
    /// starting your application with [`vial::run!`](macro.run.html).
    ///
    /// ```ignore
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    /// use vial::prelude::*;
    ///
    /// routes! {
    ///     #[filter(count)]
    ///     GET "/" => |req|
    ///         format!("Hits: {}", req.state::<Counter>.hits());
    /// }
    ///
    /// fn count(req: &mut Request) -> Option<Response> {
    ///     req.state::<Counter>().incr();
    ///     None
    /// }
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter(AtomicUsize);
    ///
    /// impl Counter {
    ///     fn hits(&self) -> usize {
    ///         self.0.load(Ordering::Relaxed)
    ///     }
    ///     fn incr(&self) {
    ///         self.0.fetch_add(1, Ordering::Relaxed);
    ///     }
    /// }
    ///
    /// fn main() {
    ///     use_state!(Counter::default());
    ///     run!();
    /// }
    /// ```
    #[must_use]
    pub fn state<T: Send + Sync + 'static>() -> &'static T {
        crate::storage::get::<T>()
    }

    #[cfg(feature = "cookies")]
    /// Get the value of a cookie sent by the client.
    #[must_use]
    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.cookies
            .iter()
            .find_map(|(k, v)| if k == name { Some(v.as_ref()) } else { None })
    }
}
