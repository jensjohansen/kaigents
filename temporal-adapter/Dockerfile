FROM golang:1.23-alpine AS builder
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o /kaigents-temporal-adapter ./cmd/kaigents-temporal-adapter

FROM gcr.io/distroless/static:nonroot
COPY --from=builder /kaigents-temporal-adapter /kaigents-temporal-adapter
ENTRYPOINT ["/kaigents-temporal-adapter"]
