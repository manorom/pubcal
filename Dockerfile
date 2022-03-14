FROM rust:1.59-alpine3.15 as builder
WORKDIR /usr/src/pubcal
COPY . .
RUN apk add --no-cache musl-dev && cargo install --path .

FROM alpine:3.15
WORKDIR /pubcal
ENV PUBCAL_CONFIG=/pubcal/config/pubcal.toml
COPY --from=builder /usr/local/cargo/bin/pubcal /pubcal/pubcal
RUN mkdir -p /pubcal/config
VOLUME /pubcal/config
ENTRYPOINT ["./pubcal"]
