package clientdevnode

type DevInfo struct {
	BuilderUrl           string `json:"builder_url"`
	SequencerApiPort     uint16 `json:"sequencer_api_port"`
	L1Url                string `json:"l1_url"`
	L1LightClientAddress string `json:"l1_light_client_address"`
}
