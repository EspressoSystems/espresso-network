package log_helper

import (
	"time"

	"github.com/benbjohnson/clock"
)

type Strategy interface {
	attempt(id string, logFunc func()) Strategy
}

type LogAfterRetryStrategy struct {
	Id          string
	Limit       uint32
	haveRetried uint32
}

func NewLogAfterRetryStrategy(id string, retryTimes uint32) LogAfterRetryStrategy {
	return LogAfterRetryStrategy{
		Id:          id,
		Limit:       retryTimes,
		haveRetried: 0,
	}
}

func (s LogAfterRetryStrategy) attempt(id string, logFunc func()) Strategy {
	if s.Id != id {
		s.Id = id
		s.haveRetried = 1
		return s
	}

	s.haveRetried += 1

	if s.haveRetried <= s.Limit {
		return s
	}

	logFunc()
	return s
}

var clk clock.Clock = clock.New()

type LogAfterDurationStrategy struct {
	Id            string
	RetryDuration time.Duration
	StartTime     time.Time
}

func NewLogAfterDurationStrategy(id string, retryDuration time.Duration) LogAfterDurationStrategy {
	return LogAfterDurationStrategy{
		Id:            id,
		RetryDuration: retryDuration,
		StartTime:     clk.Now(),
	}
}

func (s LogAfterDurationStrategy) attempt(id string, logFunc func()) Strategy {
	if s.Id != id {
		s.Id = id
		s.StartTime = clk.Now()
		return s
	}

	if clk.Since(s.StartTime) > s.RetryDuration {
		logFunc()
		return s
	}
	return s
}
