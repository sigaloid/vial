<!-- next-header -->

## [] - 

## [0.1.11] - 2021-10-23 (sigaloid/vial fork)
- Dynamically increase threadpool size when filled up [link](https://github.com/xvxx/vial/pull/10)
- Actually make cookies work (switch to different cookie crate) [instructions from MANUAL.md still apply](https://github.com/sigaloid/vial/blob/master/docs/MANUAL.md#cookies)
- Remote_addr field in request [link](https://github.com/xvxx/vial/pull/6)


## [0.1.9] - 2020-12-13

- Changed `ASSET_DIR` to store a `String` instead of `&'static str`,
  meaning it can now be set dynamically when your Vial app starts.

## [0.1.8] - 2020-11-20

- Added a lot more content types thanks to [Mozilla][moz mime types].
- Fixed parsing of HTTP headers with the same name. An HTTP client can
  now send "Accept: image/gif\r\nAccept: image/jpeg\r\n" and Vial will
  will return "image/gif, image/jpeg" from `request.header("Accept")`.

  For more information see RFC2616:

  https://greenbytes.de/tech/webdav/rfc2616.html#message.headers

[moz mime types]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types

## 0.1.7 (2020-10-21)

- Added optional `cookies` feature.

## v0.1.6

- Fix user agent timeout on empty response body.
- Fix date format in HTTP response.

Thanks to https://redbot.org and @tdryer for this release!

## v0.1.5

- Added optional `json_serde` feature with support for
  JSON via `Request::json` thanks to @tdryer!
- Removed the `state` feature. Global state is built-in.
- Added basic support for [Hatter](https://github.com/xvxx/hatter)
  HTML templates.

## v0.1.4

- Fix routing paths with fewer parts than a pattern.
- Removed the dependency on percent-encoding. Now Vial
  has only **two** direct dependencies and four total.

## v0.1.3

- Hatter now rejects headers that are over 8KB in total.
- Minor changes to HTTP header generation.

## v0.1.2

- Any panic! in app code is now converted into an error page.
- You can now disable or set your own startup banner to show
  in the console.

## v0.1.1

This release fixes a few small bugs in error handling and HTTP
parsing.

## v0.1.0

This is the first public release of **Vial**, a micro micro-framework
for the Rust programming language.

For an overview, please see [the manual][manual] or the [README][readme].

Enjoy.

[manual]: https://vial.rs
[readme]: https://github.com/xvxx/vial#readme
