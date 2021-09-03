#PIXVERT (pixvert_rs)

## About

This application is created to scale and convert images from web for SEO purposes.

It is similiar to the [Piuma Custom](https://gitlab.com/lucassith/piuma-custom) project.

Rust languages allows for more performance oriented approach than Golang.

This is not production ready yet - if you are interested in more wholesome solution - use Piuma Custom for now.

## Features

- Fetching images from external source and Cache them with ETAG and/or Modified-Since Headers.
- Decoding PNG/JPEG images.
- Scaling images to given resolution via REST parameters.
- Encoding scaled images to WEBP.

## Usage

Clone this repo and run:
```
cargo run --release
```

After that application is ready you will be able to execute HTTP request.

Example requests:

```
curl localhost:8080/
```

