// File: dashboard/internal/artifacts/proxy.go
// Purpose: Artifact proxy with support for Range headers and cloud-agnostic S3 storage.
// Product/business importance: Enables large-object streaming (video, logs, large datasets) without exposing S3 credentials.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package artifacts

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"go.uber.org/zap"
)

type Proxy struct {
	client *s3.Client
	bucket string
	logger *zap.Logger
}

type Config struct {
	Endpoint        string
	Region          string
	Bucket          string
	AccessKey       string
	SecretKey       string
	ForcePathStyle bool
}

func NewProxy(cfg Config, logger *zap.Logger) (*Proxy, error) {
	customResolver := aws.EndpointResolverWithOptionsFunc(func(service, region string, options ...interface{}) (aws.Endpoint, error) {
		if cfg.Endpoint != "" {
			return aws.Endpoint{
				URL:           cfg.Endpoint,
				SigningRegion: cfg.Region,
			}, nil
		}
		return aws.Endpoint{}, &aws.EndpointNotFoundError{}
	})

	awsCfg, err := config.LoadDefaultConfig(context.TODO(),
		config.WithRegion(cfg.Region),
		config.WithEndpointResolverWithOptions(customResolver),
		config.WithCredentialsProvider(credentials.NewStaticCredentialsProvider(cfg.AccessKey, cfg.SecretKey, "")),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to load SDK config: %w", err)
	}

	client := s3.NewFromConfig(awsCfg, func(o *s3.Options) {
		o.UsePathStyle = cfg.ForcePathStyle
	})

	return &Proxy{
		client: client,
		bucket: cfg.Bucket,
		logger: logger,
	}, nil
}

func (p *Proxy) ServeHTTP(w http.ResponseWriter, r *http.Request, key string) {
	input := &s3.GetObjectInput{
		Bucket: aws.String(p.bucket),
		Key:    aws.String(key),
	}

	// Forward Range header if present
	if rangeHeader := r.Header.Get("Range"); rangeHeader != "" {
		input.Range = aws.String(rangeHeader)
	}

	output, err := p.client.GetObject(r.Context(), input)
	if err != nil {
		p.logger.Error("failed to get object from S3", zap.String("key", key), zap.Error(err))
		if strings.Contains(err.Error(), "NoSuchKey") {
			http.NotFound(w, r)
		} else {
			http.Error(w, "Failed to retrieve artifact", http.StatusInternalServerError)
		}
		return
	}
	defer output.Body.Close()

	// Copy headers from S3 response
	if output.ContentType != nil {
		w.Header().Set("Content-Type", *output.ContentType)
	}
	if output.ContentRange != nil {
		w.Header().Set("Content-Range", *output.ContentRange)
		w.WriteHeader(http.StatusPartialContent)
	}
	if output.ContentLength != nil {
		w.Header().Set("Content-Length", fmt.Sprint(*output.ContentLength))
	}
	if output.ETag != nil {
		w.Header().Set("ETag", *output.ETag)
	}

	// Stream body
	if _, err := io.Copy(w, output.Body); err != nil {
		p.logger.Error("failed to stream S3 body to response", zap.String("key", key), zap.Error(err))
	}
}
