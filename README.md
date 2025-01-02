# Packet Observe Redirect Control Operator

## PORCOD

PORCO daemon is the service to be placed in any internet-exposed place (DMZ, hosting, cloud, etc...)

```
Usage: porcod [OPTIONS]

Options:
  -A, --grpc-addr <GRPC_ADDR>                          grpc bind address [default: 0.0.0.0:50051]
  -C, --grpc-certs <GRPC_CERTS>                        grpc public certificate (pem format)
  -K, --grpc-private-key <GRPC_PRIVATE_KEY>            grpc private key
  -a, --webserver-addr <WEBSERVER_ADDR>                webserver bind address [default: 0.0.0.0:80]
  -c, --webserver-certs <WEBSERVER_CERTS>              webserver public certificate (pem format)
  -k, --webserver-private-key <WEBSERVER_PRIVATE_KEY>  webserver private key
  -f, --webserver-filters <WEBSERVER_FILTERS>          webserver incoming filters
  -t, --webserver-timeout <WEBSERVER_TIMEOUT>          webserver timeout in seconds [default: 60]
  -h, --help                                           Print help
  -V, --version                                        Print version
```

## PORCOC

PORCO client is the service to be placed in your LAN, it will connect to PORCOD and call your internal service

```
Usage: porcoc [OPTIONS] --target-url <TARGET_URL> --porcod-url <PORCOD_URL>

Options:
  -u, --target-url <TARGET_URL>      private service url
  -U, --porcod-url <PORCOD_URL>      porco server url
  -C, --porcod-certs <PORCOD_CERTS>  grpc public certificate (pem format)
  -h, --help                         Print help
  -V, --version                      Print version
```

## Schema

```ascii
┌───────────────────────────────────────────────────┐
│                                                   │
│  Internet                                         │
│                                                   │
│                                                   │
│    ┌──────────┐   1 Call        ┌────────────┐    │
│    │          ◄─────────────────┤            │    │
│    │  PORCOD  │                 │  Caller    │    │
│    │          ├─────────────────►            │    │
│    └──┬─────▲─┘   6 Response    └────────────┘    │
│       │     │                                     │
└───────┼─────┼─────────────────────────────────────┘
2 Stream│     │5 Call                                  
┌───────┼─────┼─────────────────────────────────────┐
│       │     │                                     │
│  LAN  │     │                                     │
│       │     │                                     │
│    ┌──▼─────┴─┐    4 Response    ┌───────────┐    │
│    │          ◄──────────────────┤  Private  │    │
│    │  PORCOC  │                  │  Service  │    │
│    │          ├──────────────────►           │    │
│    └──────────┘    3 Call        └───────────┘    │
│                                                   │
│                                                   │
└───────────────────────────────────────────────────┘
```

1. An external caller calls PORCOD thinking to be calling your private service
2. PORCOC is persistently receiving a realtime stream of data from PORCOD
3. PORCOC calls your private service mimicing the caller request
4. PORCOC reads your private service response
5. PORCOC sends the response to PORCOD
6. PORCOD sends the response to the caller
