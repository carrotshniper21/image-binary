FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .
RUN  cargo build --release && mv ./target/release/image-binary ./image-binary
CMD ./image-binary
