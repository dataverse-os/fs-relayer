use std::str::FromStr;

use base64::Engine;
use dataverse_ceramic::StreamId;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessControl {
	pub encryption_provider: Option<EncryptionProvider>,
	pub monetization_provider: Option<MonetizationProvider>,
}

impl FromStr for AccessControl {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let v = base64::engine::general_purpose::STANDARD_NO_PAD.decode(s)?;
		Ok(serde_json::from_slice::<Self>(&v)?)
	}
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionProvider {
	pub protocol: EncryptionProtocol,
	pub encrypted_symmetric_key: Option<String>,
	pub decryption_conditions: Option<Vec<DecryptionCondition>>,
	pub decryption_conditions_type: Option<DecryptionConditionsTypes>,
}

impl EncryptionProvider {
	pub fn linked_ceramic_models(&self) -> anyhow::Result<Vec<StreamId>> {
		let mut models = vec![];
		if let Some(conditions) = &self.decryption_conditions {
			for ele in conditions {
				if let DecryptionCondition::AccessControl(ele) = ele {
					let model_id: StreamId = ele
						.return_value_test
						.value
						.split('=')
						.last()
						.expect("failed to parse returnValueParse.value as ceramic streamId")
						.parse()?;
					models.push(model_id);
				}
			}
		}
		Ok(models)
	}
}

#[derive(Debug, Deserialize)]
pub enum DecryptionConditionsTypes {
	AccessControlCondition,
	UnifiedAccessControlCondition,
}

#[derive(Debug, Deserialize)]
pub enum EncryptionProtocol {
	Lit,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DecryptionCondition {
	#[serde(rename_all = "camelCase")]
	AccessControl(AccessControlCondition),
	#[serde(rename_all = "camelCase")]
	Boolean(BooleanCondition),
	#[serde(rename_all = "camelCase")]
	UnifiedAccessControl(Vec<UnifiedAccessControlConditions>),

	#[serde(rename_all = "camelCase")]
	Any(serde_json::Value),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessControlCondition {
	pub condition_type: String,
	pub contract_address: String,
	pub standard_contract_type: String,
	pub chain: String,
	pub method: String,
	pub parameters: Vec<String>,
	pub return_value_test: ReturnValueTest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooleanCondition {
	pub operator: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum UnifiedAccessControlConditions {
	#[serde(rename_all = "camelCase")]
	UnifiedAccessControl(Box<UnifiedAccessControlCondition>),
	#[serde(rename_all = "camelCase")]
	Boolean(Box<BooleanCondition>),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedAccessControlCondition {
	pub contract_address: String,
	pub condition_type: String,
	pub standard_contract_type: Option<String>,
	pub method: Option<String>,
	pub parameters: Option<Vec<String>>,
	pub function_name: Option<String>,
	pub function_params: Option<Vec<String>>,
	pub chain: String,
	pub return_value_test: ReturnValueTest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReturnValueTest {
	pub key: Option<String>,
	pub comparator: String,
	pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonetizationProvider {
	pub data_asset: Option<DataAsset>,
	pub dependencies: Option<Vec<Dependence>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataAsset {
	pub asset_id: String,
	pub asset_contract: String,
	pub chain_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependence {
	pub linked_asset: DataAsset,
	pub attached: Option<Attached>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attached {
	pub block_number: Option<u64>,
}

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn decode_access_control_decryption_condition() {
		// case AccessControlCondition
		let data = serde_json::json!({
			"conditionType": "evmBasic",
			"contractAddress": "",
			"standardContractType": "SIWE",
			"chain": "ethereum",
			"method": "",
			"parameters": [
				":resources"
			],
			"returnValueTest": {
				"comparator": "contains",
				"value": "ceramic://*?model=kjzl6hvfrbw6cagt694iim2wuecu7eumeds7qd0p6uzm8dnqsq69ll7kacm05gu"
			}
		});

		let condition = serde_json::from_value::<AccessControlCondition>(data.clone());
		assert!(condition.is_ok());

		let condition = serde_json::from_value::<DecryptionCondition>(data);
		assert!(condition.is_ok());

		// case UnifiedAccessControlCondition
		let data = serde_json::json!({
			"contractAddress": "0x8673f21B34319BD0709A7a501BD0fdB614A0a7A1",
			"conditionType": "evmContract",
			"functionName": "isCollected",
			"functionParams": [
				":userAddress"
			],
			"functionAbi": {
				"inputs": [
					{
						"internalType": "address",
						"name": "user",
						"type": "address"
					}
				],
				"name": "isCollected",
				"outputs": [
					{
						"internalType": "bool",
						"name": "",
						"type": "bool"
					}
				],
				"stateMutability": "view",
				"type": "function"
			},
			"chain": "mumbai",
			"returnValueTest": {
				"key": "",
				"comparator": "=",
				"value": "true"
			}
		});

		let condition = serde_json::from_value::<UnifiedAccessControlCondition>(data.clone());
		assert!(condition.is_ok());

		let condition = serde_json::from_value::<DecryptionCondition>(data);
		assert!(condition.is_ok());

		// case UnifiedAccessControlConditions
		let data = serde_json::json!([
			{
				"conditionType": "evmBasic",
				"contractAddress": "",
				"standardContractType": "",
				"chain": "ethereum",
				"method": "",
				"parameters": [
					":userAddress"
				],
				"returnValueTest": {
					"comparator": "=",
					"value": "0x312eA852726E3A9f633A0377c0ea882086d66666"
				}
			},
			{
				"operator": "or"
			},
			{
				"contractAddress": "0x8673f21B34319BD0709A7a501BD0fdB614A0a7A1",
				"conditionType": "evmContract",
				"functionName": "isCollected",
				"functionParams": [
					":userAddress"
				],
				"functionAbi": {
					"inputs": [
						{
							"internalType": "address",
							"name": "user",
							"type": "address"
						}
					],
					"name": "isCollected",
					"outputs": [
						{
							"internalType": "bool",
							"name": "",
							"type": "bool"
						}
					],
					"stateMutability": "view",
					"type": "function"
				},
				"chain": "mumbai",
				"returnValueTest": {
					"key": "",
					"comparator": "=",
					"value": "true"
				}
			}
		]);

		let condition = serde_json::from_value::<Vec<UnifiedAccessControlConditions>>(data);
		assert!(condition.is_ok());
	}

	#[test]
	fn decode_encryption_provider() {
		let data = serde_json::json!({
		  "protocol": "Lit",
		  "encryptedSymmetricKey": "587360f2772503abd24cf63da2250005cab77d9a668552e7bd3728e8a73e30d4c46279611d29d807bfeee58c0f8d1e0dc4ba9b91c1130ae11fedebd7ec7f8934bcecddd717e24a99245596728c501db9255b8ba1f7faab19ab996f6df03ab7940eef2eede4d31821a184f9c5cbad25e5ebb1497363462ede2fff555b9704438a0000000000000020dc503f11f7c6e70c4432ef9f7ab6a3a8d805facf169d2ae6f0b662facef3a4a95d730f991ee0f28bf997eb87102000b6",
		  "decryptionConditions": [
			{
			  "conditionType": "evmBasic",
			  "contractAddress": "",
			  "standardContractType": "SIWE",
			  "chain": "ethereum",
			  "method": "",
			  "parameters": [
				":resources"
			  ],
			  "returnValueTest": {
				"comparator": "contains",
				"value": "ceramic://*?model=kjzl6hvfrbw6cagt694iim2wuecu7eumeds7qd0p6uzm8dnqsq69ll7kacm05gu"
			  }
			},
			{
			  "operator": "and"
			},
			{
			  "conditionType": "evmBasic",
			  "contractAddress": "",
			  "standardContractType": "SIWE",
			  "chain": "ethereum",
			  "method": "",
			  "parameters": [
				":resources"
			  ],
			  "returnValueTest": {
				"comparator": "contains",
				"value": "ceramic://*?model=kjzl6hvfrbw6c7gu88g66z28n81lcpbg6hu2t8pu2pui0sfnpvsrhqn3kxh9xai"
			  }
			},
			{
			  "operator": "and"
			},
			{
			  "conditionType": "evmBasic",
			  "contractAddress": "",
			  "standardContractType": "SIWE",
			  "chain": "ethereum",
			  "method": "",
			  "parameters": [
				":resources"
			  ],
			  "returnValueTest": {
				"comparator": "contains",
				"value": "ceramic://*?model=kjzl6hvfrbw6c86gt9j415yw2x8stmkotcrzpeutrbkp42i4z90gp5ibptz4sso"
			  }
			},
			{
			  "operator": "and"
			},
			{
			  "conditionType": "evmBasic",
			  "contractAddress": "",
			  "standardContractType": "SIWE",
			  "chain": "ethereum",
			  "method": "",
			  "parameters": [
				":resources"
			  ],
			  "returnValueTest": {
				"comparator": "contains",
				"value": "ceramic://*?model=kjzl6hvfrbw6catek36h3pep09k9gymfnla9k6ojlgrmwjogvjqg8q3zpybl1yu"
			  }
			},
			{
			  "operator": "and"
			},
			[
			  {
				"conditionType": "evmBasic",
				"contractAddress": "",
				"standardContractType": "",
				"chain": "ethereum",
				"method": "",
				"parameters": [
				  ":userAddress"
				],
				"returnValueTest": {
				  "comparator": "=",
				  "value": "0x312eA852726E3A9f633A0377c0ea882086d66666"
				}
			  },
			  {
				"operator": "or"
			  },
			  {
				"contractAddress": "0x8673f21B34319BD0709A7a501BD0fdB614A0a7A1",
				"conditionType": "evmContract",
				"functionName": "isCollected",
				"functionParams": [
				  ":userAddress"
				],
				"functionAbi": {
				  "inputs": [
					{
					  "internalType": "address",
					  "name": "user",
					  "type": "address"
					}
				  ],
				  "name": "isCollected",
				  "outputs": [
					{
					  "internalType": "bool",
					  "name": "",
					  "type": "bool"
					}
				  ],
				  "stateMutability": "view",
				  "type": "function"
				},
				"chain": "mumbai",
				"returnValueTest": {
				  "key": "",
				  "comparator": "=",
				  "value": "true"
				}
			  }
			]
		  ],
		  "decryptionConditionsType": "UnifiedAccessControlCondition"
		});

		let encryption_provider = serde_json::from_value::<EncryptionProvider>(data);
		assert!(encryption_provider.is_ok());
	}

	#[test]
	fn decode_access_control() -> anyhow::Result<()> {
		let data = serde_json::json!({
			"monetizationProvider": {
					"dataAsset": {
							"assetId": "0x11a8093021037a7d95d3073535c345041d3461e074525765069007de9beac8c5",
							"assetContract": "0x67804E153F9675E2173142B76f9fe949b2b20bDE",
							"chainId": 80001
					}
			},
			"encryptionProvider": {
					"protocol": "Lit",
					"encryptedSymmetricKey": "7b2263697068657274657874223a226b436630645142693838364d48464a65483077344751676c4a37472b75694c78314c2f424c777362476c4c5355376c7963745a6441545678515979767532325a635a6d5450456b4e6f2f4a36486b594f637a794c4e4c4636674679657a49505858456548526d5876637664426e6f3158736f6d5961786d315136467a78364b6135675a4a355633713350305635576e4c466e46442f6c79435276682f7370644c3543764134367773535633686258544467595a4b493550323949756c7974326b312f7344222c2264617461546f456e637279707448617368223a2264343563666237666663663834646565666365376539323439386662623365333535393338373032313065613632333233313634613831343566383566363166227d",
					"decryptionConditions": [
							{
									"conditionType": "evmBasic",
									"contractAddress": "",
									"standardContractType": "SIWE",
									"chain": "ethereum",
									"method": "",
									"parameters": [
											":resources"
									],
									"returnValueTest": {
											"comparator": "contains",
											"value": "ceramic://*?model=kjzl6hvfrbw6c89f0p1lyd1e78tel33qebisfdsi0prhhapn4rye45j1uj72tju"
									}
							},
							{
									"operator": "and"
							},
							{
									"conditionType": "evmBasic",
									"contractAddress": "",
									"standardContractType": "SIWE",
									"chain": "ethereum",
									"method": "",
									"parameters": [
											":resources"
									],
									"returnValueTest": {
											"comparator": "contains",
											"value": "ceramic://*?model=kjzl6hvfrbw6cb2cjc4cprolj8vnykf41834r9chlay1582sjxleag1b0juy5kl"
									}
							},
							{
									"operator": "and"
							},
							{
									"conditionType": "evmBasic",
									"contractAddress": "",
									"standardContractType": "SIWE",
									"chain": "ethereum",
									"method": "",
									"parameters": [
											":resources"
									],
									"returnValueTest": {
											"comparator": "contains",
											"value": "ceramic://*?model=kjzl6hvfrbw6c5m61z7cvgk4xwzx0aelqj4f9hmctn8ha64qtasd8e2779dswd5"
									}
							},
							{
									"operator": "and"
							},
							[
									[
											{
													"conditionType": "evmBasic",
													"contractAddress": "",
													"standardContractType": "",
													"chain": "ethereum",
													"method": "",
													"parameters": [
															":userAddress"
													],
													"returnValueTest": {
															"comparator": "=",
															"value": "0xCedf62df194542b3fb3E376848f87cE9afd3CdDe"
													}
											}
									],
									{
											"operator": "or"
									},
									[
											[
													{
															"contractAddress": "0x67804E153F9675E2173142B76f9fe949b2b20bDE",
															"conditionType": "evmContract",
															"chain": "mumbai",
															"functionName": "isAccessible",
															"functionAbi": {
																	"inputs": [
																			{
																					"internalType": "bytes32",
																					"name": "assetId",
																					"type": "bytes32"
																			},
																			{
																					"internalType": "address",
																					"name": "account",
																					"type": "address"
																			},
																			{
																					"internalType": "uint256",
																					"name": "tier",
																					"type": "uint256"
																			}
																	],
																	"name": "isAccessible",
																	"outputs": [
																			{
																					"internalType": "bool",
																					"name": "",
																					"type": "bool"
																			}
																	],
																	"stateMutability": "view",
																	"type": "function"
															},
															"returnValueTest": {
																	"key": "",
																	"comparator": "=",
																	"value": "true"
															},
															"functionParams": [
																	"0x11a8093021037a7d95d3073535c345041d3461e074525765069007de9beac8c5",
																	":userAddress"
															]
													}
											]
									]
							]
					],
					"decryptionConditionsType": "UnifiedAccessControlCondition"
			}
		});
		let access_control = serde_json::from_value::<AccessControl>(data);
		assert!(access_control.is_ok());

		let access_control = access_control.unwrap();
		let linked_ceramic_models = access_control
			.encryption_provider
			.unwrap()
			.linked_ceramic_models();
		assert!(linked_ceramic_models.is_ok());
		assert_eq!(
			linked_ceramic_models.unwrap(),
			vec![
				"kjzl6hvfrbw6c89f0p1lyd1e78tel33qebisfdsi0prhhapn4rye45j1uj72tju".parse()?,
				"kjzl6hvfrbw6cb2cjc4cprolj8vnykf41834r9chlay1582sjxleag1b0juy5kl".parse()?,
				"kjzl6hvfrbw6c5m61z7cvgk4xwzx0aelqj4f9hmctn8ha64qtasd8e2779dswd5".parse()?,
			]
		);
		Ok(())
	}
}
