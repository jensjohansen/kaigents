# syntax=docker/dockerfile:1

FROM golang:1.23-bookworm AS builder

WORKDIR /src

COPY go.mod go.sum ./
RUN go mod download

COPY . .

RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath -ldflags="-s -w" -o /out/kaigents-operator ./cmd/kaigents-operator

FROM gcr.io/distroless/base-debian12:nonroot

COPY --from=builder /out/kaigents-operator /usr/local/bin/kaigents-operator

ENTRYPOINT ["/usr/local/bin/kaigents-operator"]
