package common

func GetMagicBytes() [32]byte {
	return NewRawCommitmentBuilder("espresso-builder-zNC8sXSk5Yl6Uiu").Finalize()
}
