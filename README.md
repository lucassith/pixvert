# PIXVERT (pixvert_rs)

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

### Installation

Clone this repo and run:
```
cargo run --release
```

After that application is ready you will be able to execute HTTP request.

### Before you begin

Having an image resource available under: `https://via.placeholder.com/150x100`

You must first do URI Encode the string to: `https%3A%2F%2Fvia.placeholder.com%2F150x100`

### Possible image formats:

Decode: `PNG`, `JPG`

Encode: `PNG`, `JPG`, `WEBP`

## Example Requests

### Cache Image Only

If you want to cache the image, you can execute following request

```
curl localhost:8080/{encoded url}
```
example
```
curl localhost:8080/https%3A%2F%2Fvia.placeholder.com%2F150x100
```

As a response you will receive exact same image but served from cache.

### Change Format + Cache Image

You can change the file format using following request:

```
curl localhost:8080/{format}/{url}
```
example
```
curl localhost:8080/webp/https%3A%2F%2Fvia.placeholder.com%2F150x100
```

As a response you will receive webp encoded image.

### Resize + Cache Image

You can change the file format using following request:

```
curl localhost:8080/{width}_{height}/{url}
```
example
```
curl localhost:8080/100_400/https%3A%2F%2Fvia.placeholder.com%2F150x100
```

As a response you will receive a scaled image (it will not be exactly 100x400 since the image keeps the ratio)

### Resize + Change Format + Cache an image

You can change the file format using following request:

```
curl localhost:8080/{width}_{height}/webp/{url}
```
example
```
curl localhost:8080/100_400/webp/https%3A%2F%2Fvia.placeholder.com%2F150x100
```

## TODO:

- [x] Handle Cache-Control header when fetching external image.
- [x] Implement `quality` parameter.
- [x] Implement keeping aspect ratio.
- [x] Implement Cache-Control for response. 
- [x] Configuration file.
- [x] Implement JPEG-XL for encoding.
- [x] Implement JPEG-XL for decoding.
- [x] Maximum output file pixels
