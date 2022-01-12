use crate::mock::*;
use ark_crypto_primitives::snark::SNARK;
use ark_ff::{BigInteger, PrimeField, UniformRand};
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{rand::thread_rng, rc::Rc, vec::Vec};
use arkworks_circuits::{
	circuit::vanchor::VAnchorCircuit as VACircuit,
	setup::common::{
		LeafCRHGadget, PoseidonCRH_x5_2, PoseidonCRH_x5_2Gadget, PoseidonCRH_x5_3Gadget, PoseidonCRH_x5_4,
		TreeConfig_x5, Tree_x5,
	},
};
use arkworks_gadgets::{
	arbitrary::vanchor_data::VAnchorArbitraryData,
	keypair::vanchor::Keypair,
	leaf::vanchor::{Private as LeafPrivateInput, Public as LeafPublicInput, VAnchorLeaf as Leaf},
	merkle_tree::Path,
	set::membership::{Private as SetPrivateInputs, SetMembership},
};
use arkworks_utils::{
	poseidon::PoseidonParameters,
	utils::common::{
		setup_params_x5_2, setup_params_x5_3, setup_params_x5_4, setup_params_x5_5, verify_groth16, Curve,
	},
};
use codec::Encode;
use webb_primitives::{
	hashing::ethereum::keccak256,
	types::{IntoAbiToken, Token},
	utils::element_encoder,
	ElementTrait,
};

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;
type Bn254 = ark_bn254::Bn254;

const TREE_DEPTH: usize = 30;
const M: usize = 2;
const INS: usize = 2;
const OUTS: usize = 2;

#[derive(Debug)]
pub struct ExtData {
	pub recipient_bytes: Vec<u8>,
	pub relayer_bytes: Vec<u8>,
	pub ext_amount_bytes: Vec<u8>,
	pub fee_bytes: Vec<u8>,
	pub encrypted_output1_bytes: Vec<u8>,
	pub encrypted_output2_bytes: Vec<u8>,
}

impl ExtData {
	pub fn new(
		recipient_bytes: Vec<u8>,
		relayer_bytes: Vec<u8>,
		ext_amount_bytes: Vec<u8>,
		fee_bytes: Vec<u8>,
		encrypted_output1_bytes: Vec<u8>,
		encrypted_output2_bytes: Vec<u8>,
	) -> Self {
		Self {
			recipient_bytes,
			relayer_bytes,
			ext_amount_bytes,
			fee_bytes,
			encrypted_output1_bytes,
			encrypted_output2_bytes,
		}
	}
}

impl IntoAbiToken for ExtData {
	fn into_abi(&self) -> Token {
		let recipient = Token::Bytes(self.recipient_bytes.clone());
		let ext_amount = Token::Bytes(self.ext_amount_bytes.clone());
		let relayer = Token::Bytes(self.relayer_bytes.clone());
		let fee = Token::Bytes(self.fee_bytes.clone());
		let encrypted_output1 = Token::Bytes(self.encrypted_output1_bytes.clone());
		let encrypted_output2 = Token::Bytes(self.encrypted_output2_bytes.clone());
		Token::Tuple(vec![
			recipient,
			relayer,
			ext_amount,
			fee,
			encrypted_output1,
			encrypted_output2,
		])
	}
}

#[derive(Clone)]
pub struct Utxos {
	pub chain_ids: Vec<Bn254Fr>,
	pub amounts: Vec<Bn254Fr>,
	pub keypairs: Vec<Keypair<Bn254Fr, PoseidonCRH_x5_2<Bn254Fr>>>,
	pub leaf_privates: Vec<LeafPrivateInput<Bn254Fr>>,
	pub leaf_publics: Vec<LeafPublicInput<Bn254Fr>>,
	pub nullifiers: Vec<Bn254Fr>,
	pub commitments: Vec<Bn254Fr>,
}

impl Utxos {
	fn new(chain_ids: Vec<Bn254Fr>, amounts: Vec<Bn254Fr>) -> Self {
		let (params2, _, params4, params5) = get_hash_params::<Bn254Fr>(Curve::Bn254);

		let keypairs = setup_keypairs(amounts.len());
		let (commitments, nullifiers, leaf_privates, leaf_publics) =
			setup_leaves(&chain_ids, &amounts, &keypairs, &params2, &params4, &params5);

		Self {
			chain_ids,
			amounts,
			keypairs,
			leaf_privates,
			leaf_publics,
			nullifiers,
			commitments,
		}
	}

	fn new_raw(chain_ids: Vec<ChainId>, out_amounts: Vec<Balance>) -> Self {
		let chain_ids_f = chain_ids.iter().map(|x| Bn254Fr::from(*x)).collect();
		let amount_f = out_amounts.iter().map(|x| Bn254Fr::from(*x)).collect();

		Self::new(chain_ids_f, amount_f)
	}
}

pub fn get_hash_params<F: PrimeField>(
	curve: Curve,
) -> (
	PoseidonParameters<F>,
	PoseidonParameters<F>,
	PoseidonParameters<F>,
	PoseidonParameters<F>,
) {
	(
		setup_params_x5_2::<F>(curve),
		setup_params_x5_3::<F>(curve),
		setup_params_x5_4::<F>(curve),
		setup_params_x5_5::<F>(curve),
	)
}

pub fn setup_random_circuit() -> VACircuit<
	Bn254Fr,
	PoseidonCRH_x5_2<Bn254Fr>,
	PoseidonCRH_x5_2Gadget<Bn254Fr>,
	TreeConfig_x5<Bn254Fr>,
	LeafCRHGadget<Bn254Fr>,
	PoseidonCRH_x5_3Gadget<Bn254Fr>,
	TREE_DEPTH,
	INS,
	OUTS,
	M,
> {
	let rng = &mut thread_rng();

	let public_amount = Bn254Fr::rand(rng);
	let recipient = Bn254Fr::rand(rng);
	let relayer = Bn254Fr::rand(rng);
	let ext_amount = Bn254Fr::rand(rng);
	let fee = Bn254Fr::rand(rng);

	let in_chain_id = Bn254Fr::rand(rng);
	let in_amounts = vec![Bn254Fr::rand(rng); INS];
	let out_chain_ids = vec![Bn254Fr::rand(rng); OUTS];
	let out_amounts = vec![Bn254Fr::rand(rng); OUTS];

	let (circuit, ..) = setup_circuit_with_data(
		public_amount,
		recipient.into_repr().to_bytes_le(),
		relayer.into_repr().to_bytes_le(),
		ext_amount.into_repr().to_bytes_le(),
		fee.into_repr().to_bytes_le(),
		in_chain_id,
		in_amounts,
		out_chain_ids,
		out_amounts,
	);

	circuit
}

pub fn setup_circuit_with_data_raw(
	// Metadata inputs
	public_amount: Amount,
	recipient: AccountId,
	relayer: AccountId,
	ext_amount: Amount,
	fee: Balance,
	// Transaction inputs
	in_chain_id: ChainId,
	in_amounts: Vec<Balance>,
	// Transaction outputs
	out_chain_ids: Vec<ChainId>,
	out_amounts: Vec<Balance>,
) -> (
	VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
	Vec<Bn254Fr>,
	Utxos,
	Utxos,
) {
	let chain_id_bytes = in_chain_id.using_encoded(element_encoder);
	let in_chain_id_f = Bn254Fr::from_le_bytes_mod_order(&chain_id_bytes);

	let public_amount_f = Bn254Fr::from(public_amount);

	let in_amounts_f = in_amounts.iter().map(|x| Bn254Fr::from(*x)).collect();
	let out_chain_ids_f = out_chain_ids.iter().map(|x| Bn254Fr::from(*x)).collect();
	let out_amounts_f = out_amounts.iter().map(|x| Bn254Fr::from(*x)).collect();

	let (circuit, public_inputs_f, in_utxos, out_utxos) = setup_circuit_with_data(
		public_amount_f,
		recipient.encode(),
		relayer.encode(),
		ext_amount.encode(),
		fee.encode(),
		in_chain_id_f,
		in_amounts_f,
		out_chain_ids_f,
		out_amounts_f,
	);

	(circuit, public_inputs_f, in_utxos, out_utxos)
}

pub fn setup_circuit_with_input_utxos_raw(
	// Metadata inputs
	public_amount: Amount,
	recipient: AccountId,
	relayer: AccountId,
	ext_amount: Amount,
	fee: Balance,
	// Transaction inputs
	in_utxos: Utxos,
	// Transaction outputs
	out_chain_ids: Vec<ChainId>,
	out_amounts: Vec<Balance>,
) -> (
	VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
	Vec<Bn254Fr>,
	Utxos,
) {
	let public_amount_f = Bn254Fr::from(public_amount);

	let out_chain_ids_f = out_chain_ids.iter().map(|x| Bn254Fr::from(*x)).collect();
	let out_amounts_f = out_amounts.iter().map(|x| Bn254Fr::from(*x)).collect();

	let (circuit, public_inputs_f, out_utxos) = setup_circuit_with_input_utxos(
		public_amount_f,
		recipient.encode(),
		relayer.encode(),
		ext_amount.encode(),
		fee.encode(),
		in_utxos,
		out_chain_ids_f,
		out_amounts_f,
	);

	(circuit, public_inputs_f, out_utxos)
}

pub fn setup_circuit_with_input_utxos(
	public_amount: Bn254Fr,
	recipient: Vec<u8>,
	relayer: Vec<u8>,
	ext_amount: Vec<u8>,
	fee: Vec<u8>,
	// Input transactions
	in_utxos: Utxos,
	// Output data
	out_chain_ids: Vec<Bn254Fr>,
	out_amounts: Vec<Bn254Fr>,
) -> (
	VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
	Vec<Bn254Fr>,
	Utxos,
) {
	let (params2, params3, params4, params5) = get_hash_params::<Bn254Fr>(Curve::Bn254);

	// Tree + set for proving input txos
	let (in_paths, in_indices, in_root_set, in_set_private_inputs) =
		setup_tree_and_set(&in_utxos.commitments, &params3);

	// Output leaves (txos)
	let out_utxos = Utxos::new(out_chain_ids, out_amounts);

	let ext_data = ExtData::new(
		recipient,
		relayer,
		ext_amount,
		fee,
		out_utxos.commitments[0].into_repr().to_bytes_le(),
		out_utxos.commitments[1].into_repr().to_bytes_le(),
	);
	let ext_data_hash = keccak256(&ext_data.encode_abi());
	let ext_data_hash_f = Bn254Fr::from_le_bytes_mod_order(&ext_data_hash);
	// Arbitrary data
	let arbitrary_data = setup_arbitrary_data(ext_data_hash_f);

	let circuit = setup_circuit(
		public_amount,
		arbitrary_data,
		in_utxos.clone(),
		in_indices,
		in_paths,
		in_set_private_inputs,
		in_root_set,
		out_utxos.clone(),
		params2,
		params4,
		params5,
	);

	let public_inputs = construct_public_inputs(
		in_utxos.leaf_publics[0].chain_id,
		public_amount,
		in_root_set.to_vec(),
		in_utxos.nullifiers,
		out_utxos.commitments.clone(),
		ext_data_hash_f,
	);

	(circuit, public_inputs, out_utxos)
}

// This function is used only for first transaction, when the tree is empty
pub fn setup_circuit_with_data(
	public_amount: Bn254Fr,
	recipient: Vec<u8>,
	relayer: Vec<u8>,
	ext_amount: Vec<u8>,
	fee: Vec<u8>,
	in_chain_id: Bn254Fr,
	in_amounts: Vec<Bn254Fr>,
	out_chain_ids: Vec<Bn254Fr>,
	out_amounts: Vec<Bn254Fr>,
) -> (
	VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
	Vec<Bn254Fr>,
	Utxos,
	Utxos,
) {
	let (params2, params3, params4, params5) = get_hash_params::<Bn254Fr>(Curve::Bn254);

	// Making a vec of same chain ids to be passed into setup_leaves
	let in_chain_ids = vec![in_chain_id; in_amounts.len()];

	// Input leaves (txos)
	let in_utxos = Utxos::new(in_chain_ids, in_amounts);

	// Tree + set for proving input txos
	let (in_paths, in_indices, _, in_set_private_inputs) = setup_tree_and_set(&in_utxos.commitments, &params3);
	// Since on chain tree is empty we set the roots to zero
	let in_root_set = [Bn254Fr::from(0u32); M];

	// Output leaves (txos)
	let out_utxos = Utxos::new(out_chain_ids, out_amounts);

	let ext_data = ExtData::new(
		recipient,
		relayer,
		ext_amount,
		fee,
		out_utxos.commitments[0].into_repr().to_bytes_le(),
		out_utxos.commitments[1].into_repr().to_bytes_le(),
	);

	let ext_data_hash = keccak256(&ext_data.encode_abi());
	let ext_data_hash_f = Bn254Fr::from_le_bytes_mod_order(&ext_data_hash);
	// Arbitrary data
	let arbitrary_data = setup_arbitrary_data(ext_data_hash_f);

	let circuit = setup_circuit(
		public_amount,
		arbitrary_data,
		in_utxos.clone(),
		in_indices,
		in_paths,
		in_set_private_inputs,
		in_root_set,
		out_utxos.clone(),
		params2,
		params4,
		params5,
	);

	let public_inputs = construct_public_inputs(
		in_utxos.leaf_publics[0].chain_id,
		public_amount,
		in_root_set.to_vec(),
		in_utxos.nullifiers.clone(),
		out_utxos.commitments.clone(),
		ext_data_hash_f,
	);

	(circuit, public_inputs, in_utxos, out_utxos)
}

pub fn setup_circuit(
	public_amount: Bn254Fr,
	arbitrary_data: VAnchorArbitraryData<Bn254Fr>,
	// Input transactions
	in_utxos: Utxos,
	// Data related to tree
	in_indicies: Vec<Bn254Fr>,
	in_paths: Vec<Path<TreeConfig_x5<Bn254Fr>, TREE_DEPTH>>,
	in_set_private_inputs: Vec<SetPrivateInputs<Bn254Fr, M>>,
	in_root_set: [Bn254Fr; M],
	// Output transactions
	out_utxos: Utxos,
	// Hash function parameters
	params2: PoseidonParameters<Bn254Fr>,
	params4: PoseidonParameters<Bn254Fr>,
	params5: PoseidonParameters<Bn254Fr>,
) -> VACircuit<
	Bn254Fr,
	PoseidonCRH_x5_2<Bn254Fr>,
	PoseidonCRH_x5_2Gadget<Bn254Fr>,
	TreeConfig_x5<Bn254Fr>,
	LeafCRHGadget<Bn254Fr>,
	PoseidonCRH_x5_3Gadget<Bn254Fr>,
	TREE_DEPTH,
	INS,
	OUTS,
	M,
> {
	let out_pub_keys = out_utxos
		.keypairs
		.iter()
		.map(|x| x.public_key(&params2).unwrap())
		.collect();

	let circuit = VACircuit::<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>::new(
		public_amount,
		arbitrary_data,
		in_utxos.leaf_privates,
		in_utxos.keypairs,
		in_utxos.leaf_publics[0].clone(),
		in_set_private_inputs,
		in_root_set,
		params2,
		params4,
		params5,
		in_paths,
		in_indicies,
		in_utxos.nullifiers.clone(),
		out_utxos.commitments.clone(),
		out_utxos.leaf_privates,
		out_utxos.leaf_publics,
		out_pub_keys,
	);

	circuit
}

pub fn setup_keys(
	circuit: VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
) -> (Vec<u8>, Vec<u8>) {
	let rng = &mut thread_rng();
	let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), rng).unwrap();

	let mut pk_bytes = Vec::new();
	let mut vk_bytes = Vec::new();
	pk.serialize(&mut pk_bytes).unwrap();
	vk.serialize(&mut vk_bytes).unwrap();
	(pk_bytes, vk_bytes)
}

pub fn prove(
	circuit: VACircuit<
		Bn254Fr,
		PoseidonCRH_x5_2<Bn254Fr>,
		PoseidonCRH_x5_2Gadget<Bn254Fr>,
		TreeConfig_x5<Bn254Fr>,
		LeafCRHGadget<Bn254Fr>,
		PoseidonCRH_x5_3Gadget<Bn254Fr>,
		TREE_DEPTH,
		INS,
		OUTS,
		M,
	>,
	pk_bytes: &[u8],
) -> Vec<u8> {
	let rng = &mut thread_rng();
	let pk = ProvingKey::<ark_bn254::Bn254>::deserialize(pk_bytes).unwrap();

	let proof = Groth16::prove(&pk, circuit, rng).unwrap();
	let mut proof_bytes = Vec::new();
	proof.serialize(&mut proof_bytes).unwrap();
	proof_bytes
}

pub fn verify(public_inputs: &Vec<Bn254Fr>, vk: &[u8], proof: &[u8]) -> bool {
	let vk = VerifyingKey::<Bn254>::deserialize(vk).unwrap();
	let proof = Proof::<Bn254>::deserialize(proof).unwrap();
	let ver_res = verify_groth16(&vk, &public_inputs, &proof);
	ver_res
}

pub fn setup_keypairs(n: usize) -> Vec<Keypair<Bn254Fr, PoseidonCRH_x5_2<Bn254Fr>>> {
	let rng = &mut thread_rng();

	let mut keypairs = Vec::new();
	for _ in 0..n {
		let priv_key = Bn254Fr::rand(rng);
		let keypair = Keypair::<_, PoseidonCRH_x5_2<Bn254Fr>>::new(priv_key);
		keypairs.push(keypair);
	}
	keypairs
}

pub fn setup_leaves(
	chain_ids: &Vec<Bn254Fr>,
	amounts: &Vec<Bn254Fr>,
	keypairs: &Vec<Keypair<Bn254Fr, PoseidonCRH_x5_2<Bn254Fr>>>,
	params2: &PoseidonParameters<Bn254Fr>,
	params4: &PoseidonParameters<Bn254Fr>,
	params5: &PoseidonParameters<Bn254Fr>,
) -> (
	Vec<Bn254Fr>,
	Vec<Bn254Fr>,
	Vec<LeafPrivateInput<Bn254Fr>>,
	Vec<LeafPublicInput<Bn254Fr>>,
) {
	let rng = &mut thread_rng();

	let num_inputs = amounts.len();

	let mut leaves = Vec::new();
	let mut nullifiers = Vec::new();
	let mut private_inputs = Vec::new();
	let mut public_inputs = Vec::new();

	for i in 0..num_inputs {
		let chain_id = Bn254Fr::from(chain_ids[i]);
		let amount = Bn254Fr::from(amounts[i]);
		let blinding = Bn254Fr::rand(rng);
		let index = Bn254Fr::from(i as u64);

		let private_input = LeafPrivateInput::<Bn254Fr>::new(amount, blinding);
		let public_input = LeafPublicInput::<Bn254Fr>::new(chain_id);

		let pub_key = keypairs[i].public_key(&params2).unwrap();

		let leaf =
			Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>>::create_leaf(&private_input, &public_input, &pub_key, &params5)
				.unwrap();

		let signature = keypairs[i].signature(&leaf, &index, &params4).unwrap();

		let nullfier =
			Leaf::<Bn254Fr, PoseidonCRH_x5_4<Bn254Fr>>::create_nullifier(&signature, &leaf, &params4, &index).unwrap();

		leaves.push(leaf);
		nullifiers.push(nullfier);
		private_inputs.push(private_input);
		public_inputs.push(public_input);
	}

	(leaves, nullifiers, private_inputs, public_inputs)
}

pub fn setup_tree(
	leaves: &Vec<Bn254Fr>,
	params3: &PoseidonParameters<Bn254Fr>,
) -> (Vec<Path<TreeConfig_x5<Bn254Fr>, TREE_DEPTH>>, Vec<Bn254Fr>, Bn254Fr) {
	let inner_params = Rc::new(params3.clone());
	let tree = Tree_x5::new_sequential(inner_params, Rc::new(()), &leaves).unwrap();
	let root = tree.root();

	let num_leaves = leaves.len();

	let mut paths = Vec::new();
	let mut indices = Vec::new();
	for i in 0..num_leaves {
		let path = tree.generate_membership_proof::<TREE_DEPTH>(i as u64);
		let index = path.get_index(&root, &leaves[i]).unwrap();
		paths.push(path);
		indices.push(index);
	}

	(paths, indices, root.inner())
}

pub fn setup_root_set(root: Bn254Fr) -> ([Bn254Fr; M], Vec<SetPrivateInputs<Bn254Fr, M>>) {
	let root_set = [root.clone(); M];

	let mut set_private_inputs = Vec::new();
	for _ in 0..M {
		let set_private_input = SetMembership::generate_secrets(&root, &root_set).unwrap();
		set_private_inputs.push(set_private_input);
	}

	(root_set, set_private_inputs)
}

pub fn setup_tree_and_set(
	leaves: &Vec<Bn254Fr>,
	params3: &PoseidonParameters<Bn254Fr>,
) -> (
	Vec<Path<TreeConfig_x5<Bn254Fr>, TREE_DEPTH>>,
	Vec<Bn254Fr>,
	[Bn254Fr; M],
	Vec<SetPrivateInputs<Bn254Fr, M>>,
) {
	let (paths, indices, root) = setup_tree(&leaves, params3);
	let (root_set, set_private_inputs) = setup_root_set(root);
	(paths, indices, root_set, set_private_inputs)
}

pub fn setup_arbitrary_data(ext_data: Bn254Fr) -> VAnchorArbitraryData<Bn254Fr> {
	VAnchorArbitraryData::new(ext_data)
}

pub fn construct_public_inputs(
	chain_id: Bn254Fr,
	public_amount: Bn254Fr,
	roots: Vec<Bn254Fr>,
	nullifiers: Vec<Bn254Fr>,
	commitments: Vec<Bn254Fr>,
	ext_data_hash: Bn254Fr,
) -> Vec<Bn254Fr> {
	let mut public_inputs = vec![public_amount, ext_data_hash];
	public_inputs.extend(nullifiers);
	public_inputs.extend(commitments);
	public_inputs.push(chain_id);
	public_inputs.extend(roots);

	public_inputs
}

pub fn construct_public_inputs_el(public_inputs: &Vec<Bn254Fr>) -> Vec<Element> {
	let public_inputs_el = public_inputs
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();

	public_inputs_el
}

pub fn deconstruct_public_inputs(
	public_inputs: &Vec<Bn254Fr>,
) -> (
	Bn254Fr,      // Chain Id
	Bn254Fr,      // Public amount
	Vec<Bn254Fr>, // Roots
	Vec<Bn254Fr>, // Input tx Nullifiers
	Vec<Bn254Fr>, // Output tx commitments
	Bn254Fr,      // External data hash
) {
	let public_amount = public_inputs[0];
	let ext_data_hash = public_inputs[1];
	let nullifiers = public_inputs[2..4].to_vec();
	let commitments = public_inputs[4..6].to_vec();
	let chain_id = public_inputs[6];
	let root_set = public_inputs[7..9].to_vec();
	(
		chain_id,
		public_amount,
		root_set,
		nullifiers,
		commitments,
		ext_data_hash,
	)
}

pub fn deconstruct_public_inputs_el(
	public_inputs_f: &Vec<Bn254Fr>,
) -> (
	Element,      // Chain Id
	Element,      // Public amount
	Vec<Element>, // Roots
	Vec<Element>, // Input tx Nullifiers
	Vec<Element>, // Output tx commitments
	Element,      // External amount
) {
	let (chain_id, public_amount, roots, nullifiers, commitments, ext_data_hash) =
		deconstruct_public_inputs(public_inputs_f);
	let chain_id_el = Element::from_bytes(&chain_id.into_repr().to_bytes_le());
	let public_amount_el = Element::from_bytes(&public_amount.into_repr().to_bytes_le());
	let root_set_el = roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let nullifiers_el = nullifiers
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let commitments_el = commitments
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_le()))
		.collect();
	let ext_data_hash_el = Element::from_bytes(&ext_data_hash.into_repr().to_bytes_le());
	(
		chain_id_el,
		public_amount_el,
		root_set_el,
		nullifiers_el,
		commitments_el,
		ext_data_hash_el,
	)
}

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[12..].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
