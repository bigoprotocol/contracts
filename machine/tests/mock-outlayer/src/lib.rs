use near_sdk::{near, PanicOnDefault};

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct OutlayerResult {
    pub value: u64,
    pub signature_hex: String,
    pub alpha: String,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct OutlayerEnvelope {
    pub results: Vec<OutlayerResult>,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub alpha: String,
    pub signature_hex: String,
}

#[near]
impl Contract {
    #[init]
    pub fn new(alpha: String, signature_hex: String) -> Self {
        Self { alpha, signature_hex }
    }

    pub fn set_result(&mut self, alpha: String, signature_hex: String) {
        self.alpha = alpha;
        self.signature_hex = signature_hex;
    }

    // returns the fixed envelope, the caller's callback receives it as the promise result
    #[payable]
    pub fn request_execution(
        &mut self,
        source: near_sdk::serde_json::Value,
        resource_limits: near_sdk::serde_json::Value,
        input_data: String,
        response_format: String,
    ) -> OutlayerEnvelope {
        let _ = (source, resource_limits, input_data, response_format);
        OutlayerEnvelope {
            results: vec![OutlayerResult {
                value: 0,
                signature_hex: self.signature_hex.clone(),
                alpha: self.alpha.clone(),
            }],
        }
    }
}
