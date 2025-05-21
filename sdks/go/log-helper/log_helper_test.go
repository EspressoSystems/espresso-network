package log_helper

import (
	"testing"
	"time"

	"github.com/benbjohnson/clock"
)

func TestLogAfterRetry(t *testing.T) {
	helper := NewLogger()

	channel := Channel("test1")
	id := "id1"
	retryTimes := 5
	helper.AddLogAfterRetryStrategy(channel, id, uint32(retryTimes))

	cnt := 0
	for i := 0; i < retryTimes; i += 1 {
		helper.Attempt(channel, id, func() { cnt += 1 })
		if cnt > 0 {
			t.Error("should be zero")
		}
	}

	expect := 10
	helper.Attempt(channel, id, func() { cnt += expect })
	if cnt != expect {
		t.Errorf("expect %d, but got %d", expect, cnt)
	}

	// A new id should set the haveRetried to 1.
	cnt = 0
	id2 := "id2"
	for i := 0; i < retryTimes; i += 1 {
		helper.Attempt(channel, id2, func() { cnt += 1 })
		if cnt > 0 {
			t.Error("should be zero")
		}
	}

	helper.Attempt(channel, id2, func() { cnt += expect })
	if cnt != expect {
		t.Errorf("expect %d, but got %d", expect, cnt)
	}

}

func TestLogAfterDuration(t *testing.T) {
	mockClock := clock.NewMock()
	clk = mockClock
	helper := NewLogger()

	channel := Channel("test1")
	id := "id1"

	cnt := 0

	retryDuration := time.Hour
	helper.AddLogAfterDurationStrategy(channel, id, retryDuration)
	for i := 0; i < 5; i += 1 {
		helper.Attempt(channel, id, func() { cnt += 1 })
		if cnt > 0 {
			t.Error("should be zero")
		}
	}

	mockClock.Add(retryDuration + time.Minute)
	expect := 10
	helper.Attempt(channel, id, func() { cnt += expect })
	if cnt != expect {
		t.Errorf("expect %d, but got %d", expect, cnt)
	}

	// A new id should set the startTime to now.
	cnt = 0

	id2 := "id2"
	for i := 0; i < 5; i += 1 {
		helper.Attempt(channel, id2, func() { cnt += 1 })
		if cnt > 0 {
			t.Error("should be zero")
		}
	}

	mockClock.Add(retryDuration + time.Minute)
	helper.Attempt(channel, id2, func() { cnt += expect })
	if cnt != expect {
		t.Errorf("expect %d, but got %d", expect, cnt)
	}
}
