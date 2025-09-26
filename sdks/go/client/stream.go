package client

import (
	"context"
	"encoding/json"
	"fmt"
	"time"
)

var _ Stream[any] = (*StreamWithTimeout[any])(nil)

type StreamWithTimeout[T any] struct {
	stream  Stream[T]
	timeout time.Duration
}

func NewStreamWithTimeout[T any](stream Stream[T], timeout time.Duration) *StreamWithTimeout[T] {
	return &StreamWithTimeout[T]{
		stream:  stream,
		timeout: timeout,
	}
}

func (s *StreamWithTimeout[T]) Next(ctx context.Context) (*T, error) {
	raw, err := s.NextRaw(ctx)

	if err != nil {
		return nil, err
	}

	var data T
	if err := json.Unmarshal(raw, &data); err != nil {
		return nil, fmt.Errorf("%w: %s", ErrPermanent, err)
	}

	return &data, nil
}

func (s *StreamWithTimeout[T]) Close() error {
	result := make(chan error, 1)
	go func() {
		err := s.stream.Close()
		result <- err
	}()

	select {
	case <-time.After(s.timeout):
		return fmt.Errorf("%w: timeout after %s", ErrPermanent, s.timeout)
	case err := <-result:
		return err
	}
}

func (s *StreamWithTimeout[T]) NextRaw(ctx context.Context) (json.RawMessage, error) {
	type Result struct {
		Data json.RawMessage
		Err  error
	}

	result := make(chan Result, 1)

	go func() {
		data, err := s.stream.NextRaw(ctx)
		result <- Result{Data: data, Err: err}
	}()

	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	case <-time.After(s.timeout):
		return nil, fmt.Errorf("%w: timeout after %s", ErrTimeout, s.timeout)
	case res := <-result:
		return res.Data, res.Err
	}
}
