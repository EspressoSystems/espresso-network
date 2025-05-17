package client

// This interface represents the full API of clients defined in this package
// It is provided for consumers that wish to use the full scope of the EspressoClient
// while still providing the ability to use mock structures for testing.

type EspressoClient interface {
	QueryService
	SubmitAPI
}
