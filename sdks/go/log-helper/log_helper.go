package log_helper

import (
	"time"
)

type Channel string

// This is used for cleaning up log outputs.
// The scenario is that rollup will query hotshot blocks that are not yet ready,
// which generates a lot of logs. However, these logs don't contain much useful
// information and actually make debugging more difficult.
type LogHelper struct {
	strategies map[Channel]Strategy
}

func NewLogger() LogHelper {
	return LogHelper{strategies: map[Channel]Strategy{}}
}

func (l *LogHelper) AddStrategy(channel Channel, strategy Strategy) {
	l.strategies[channel] = strategy
}

func (l *LogHelper) AddLogAfterRetryStrategy(channel Channel, id string, retryTimes uint32) {
	s := NewLogAfterRetryStrategy(id, retryTimes)
	l.AddStrategy(channel, s)
}

func (l *LogHelper) AddLogAfterDurationStrategy(channel Channel, id string, duration time.Duration) {
	s := NewLogAfterDurationStrategy(id, duration)
	l.AddStrategy(channel, s)
}

func (l *LogHelper) Attempt(channel Channel, id string, logFunc func()) {
	strategy, ok := l.strategies[channel]
	if !ok {
		// Can not find the strategy, log it
		logFunc()
		return
	}

	result := strategy.attempt(id, logFunc)
	l.strategies[channel] = result
}
