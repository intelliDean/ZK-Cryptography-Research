use crate::circuit::Circuit;
use crate::gate::Ops;
use crate::gkr_prover::GKRProof;
use ark_ff::{BigInteger, PrimeField};
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
use sha3::{Digest, Keccak256};
use sumcheck_protocol::gkr_sumcheck::verify as sub_verify;
use sumcheck_protocol::transcript::{to_bytes, HashTrait, Transcript};


pub fn verify<F: PrimeField>(proof: GKRProof<F>, mut circuit: Circuit<F>, inputs: &[F]) -> bool {
    let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
    let mut state = VerifierState::new();

    // initial setup
    transcript.absorb(&to_bytes(&proof.output_poly.polynomial));
    let init_challenge = transcript.generate_random_challenge();
    state.current_claim = proof.output_poly.clone().full_evaluation(vec![init_challenge]);
    transcript.absorb(&to_bytes(&[state.current_claim]));

    let num_layers = circuit.layers.len();

    // verify each layer
    for i in 0..num_layers {
        let sum_check_verify = sub_verify(
            proof.proof_polynomials[i].clone(),
            state.current_claim,
            &mut transcript,
        );

        if !sum_check_verify.is_proof_valid {
            return false;
        }

        if !state.update(
            &proof,
            &mut circuit,
            inputs,
            i,
            &sum_check_verify.random_challenges,
            init_challenge,
            sum_check_verify.last_claimed_sum,
            &mut transcript,
        ) {
            return false;
        }
    }
    true
}



struct VerifierState<F: PrimeField> {
    alpha: F,
    beta: F,
    prev_challenges: Vec<F>,
    current_claim: F,
}

impl<F: PrimeField> VerifierState<F> {
    fn new() -> Self {
        Self {
            alpha: F::zero(),
            beta: F::zero(),
            prev_challenges: Vec::new(),
            current_claim: F::zero(),
        }
    }

    fn update(
        &mut self,
        proof: &GKRProof<F>,
        circuit: &mut Circuit<F>,
        inputs: &[F],
        i: usize,
        current_challenges: &[F],
        init_challenge: F,
        last_claim: F,
        transcript: &mut Transcript<Keccak256, F>,
    ) -> bool {
        let (o_1, o_2) = if i == circuit.layers.len() - 1 {
            evaluate_input(inputs, current_challenges)
        } else {
            proof.claimed_evaluations[i]
        };

        let expected_claim = if i == 0 {
            verifier_generates_claim(circuit.clone(), i, init_challenge, current_challenges, o_1, o_2)
        } else {
            verifier_claim_with_alpha_beta(
                circuit.clone(),
                i,
                current_challenges,
                &self.prev_challenges,
                o_1,
                o_2,
                self.alpha,
                self.beta,
            )
        };

        if expected_claim != last_claim {
            return false;
        }

        self.prev_challenges = current_challenges.to_vec();
        transcript.absorb(&to_bytes(&[o_1]));
        self.alpha = transcript.generate_random_challenge();
        transcript.absorb(&to_bytes(&[o_2]));
        self.beta = transcript.generate_random_challenge();
        self.current_claim = self.alpha * o_1 + self.beta * o_2;

        true
    }
}

fn verifier_generates_claim<F: PrimeField>(
    mut circuit: Circuit<F>,
    layer_idx: usize,
    init_random_challenge: F,
    sumcheck_random_challenges: &[F],
    o_1: F,
    o_2: F,
) -> F {
    let mut all_random_challenges = Vec::with_capacity(1 + sumcheck_random_challenges.len());
    all_random_challenges.push(init_random_challenge);
    all_random_challenges.extend_from_slice(sumcheck_random_challenges);

    let (add_i, mul_i) = circuit.add_i_and_mul_i_mle(layer_idx);
    let a_r = add_i.clone().full_evaluation(all_random_challenges.clone());
    let m_r = mul_i.full_evaluation(all_random_challenges);

    (a_r * (o_1 + o_2)) + (m_r * (o_1 * o_2))
}

fn verifier_claim_with_alpha_beta<F: PrimeField>(
    mut circuit: Circuit<F>,
    layer_idx: usize,
    current_random_challenge: &[F],
    previous_random_challenge: &[F],
    o_1: F,
    o_2: F,
    alpha: F,
    beta: F,
) -> F {
    let (prev_r_b, prev_r_c) = previous_random_challenge.split_at(previous_random_challenge.len() / 2);
    let (add_i, mul_i) = circuit.add_i_and_mul_i_mle(layer_idx);

    let new_add_i = add_i.multi_partial_evaluate(prev_r_b).multiply_by(alpha)
        + add_i.multi_partial_evaluate(prev_r_c).multiply_by(beta);
    let new_mul_i = mul_i.multi_partial_evaluate(prev_r_b).multiply_by(alpha)
        + mul_i.multi_partial_evaluate(prev_r_c).multiply_by(beta);

    let a_r = new_add_i.full_evaluation(current_random_challenge.to_vec());
    let m_r = new_mul_i.full_evaluation(current_random_challenge.to_vec());

    (a_r * (o_1 + o_2)) + (m_r * (o_1 * o_2))
}

fn evaluate_input<F: PrimeField>(inputs: &[F], sumcheck_random_challenges: &[F]) -> (F, F) {
    let input_poly = MultilinearPoly::new(inputs.to_vec());
    let (r_b, r_c) = sumcheck_random_challenges.split_at(sumcheck_random_challenges.len() / 2);

    let o_1 = input_poly.clone().full_evaluation(r_b.to_vec());
    let o_2 = input_poly.full_evaluation(r_c.to_vec());

    (o_1, o_2)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gate::{Gate, Ops};
    use crate::gkr_prover::prove;
    use crate::layer::Layer;
    use ark_bn254::{Fq, Fr};
    use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};

    fn get_circuit() -> Circuit<Fr> {
        let layer0 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL)]);

        let layer1 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL), Gate::new(1, 2, 3, Ops::ADD)]);

        let layer2 = Layer::new( vec![
            Gate::new(0, 0, 1, Ops::MUL),
            Gate::new(1, 2, 3, Ops::ADD),
            Gate::new(2, 4, 5, Ops::ADD),
            Gate::new(3, 6, 7, Ops::MUL),
        ]);

        let circuit = vec![layer0, layer1, layer2];


        println!("Layers: {:?}", &circuit);

        Circuit::new(circuit)
    }
    fn get_circuit1() -> Circuit<Fr> {
        let layer0 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL)]);

        let layer1 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL), Gate::new(1, 2, 3, Ops::ADD)]);

        let layer2 = Layer::new( vec![
            Gate::new(0, 0, 1, Ops::MUL),
            Gate::new(1, 1, 3, Ops::ADD),
            Gate::new(2, 4, 5, Ops::ADD),
            Gate::new(3, 6, 7, Ops::MUL),
        ]);

        let circuit = vec![layer0, layer1, layer2];


        println!("Layers: {:?}", &circuit);

        Circuit::new(circuit)
    }

    fn get_input() -> MultilinearPoly<Fr> {
        MultilinearPoly::new(vec![
            Fr::from(1),
            Fr::from(2),
            Fr::from(3),
            Fr::from(4),
            Fr::from(5),
            Fr::from(6),
            Fr::from(7),
            Fr::from(8),
        ])
    }

    #[test]
    fn it_add_polys_correctly() {
        let poly_a = &[Fq::from(0), Fq::from(2)];
        let poly_b = &[Fq::from(0), Fq::from(3)];

        let expected_poly = vec![Fq::from(0), Fq::from(3), Fq::from(2), Fq::from(5)];

        let result = Ops::ADD.cartesian_operations(poly_a, poly_b);

        assert_eq!(result.polynomial, expected_poly);

        let poly_a = &[Fq::from(0), Fq::from(3)];
        let poly_b = &[Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)];

        let expected_poly = vec![
            Fq::from(0),
            Fq::from(0),
            Fq::from(0),
            Fq::from(2),
            Fq::from(3),
            Fq::from(3),
            Fq::from(3),
            Fq::from(5),
        ];

        let result =   Ops::ADD.cartesian_operations(poly_a, poly_b);

        assert_eq!(result.polynomial, expected_poly);
    }

    #[test]
    fn it_multiplies_polys_correctly() {
        let poly_a = &[Fq::from(0), Fq::from(2)];
        let poly_b = &[Fq::from(0), Fq::from(3)];

        let expected_poly = vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(6)];

        let result = Ops::MUL.cartesian_operations(poly_a, poly_b);

        assert_eq!(result.polynomial, expected_poly);

        let poly_a = &[Fq::from(0), Fq::from(3)];
        let poly_b = &[Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)];

        let expected_poly = vec![
            Fq::from(0),
            Fq::from(0),
            Fq::from(0),
            Fq::from(0),
            Fq::from(0),
            Fq::from(0),
            Fq::from(0),
            Fq::from(6),
        ];

        let result = Ops::MUL.cartesian_operations(poly_a, poly_b);

        assert_eq!(result.polynomial, expected_poly);
    }

    #[test]
    fn test_gkr_protocol() {

        let mut circuit = get_circuit();
        let input = [
            Fr::from(1),
            Fr::from(2),
            Fr::from(3),
            Fr::from(4),
            Fr::from(5),
            Fr::from(6),
            Fr::from(7),
            Fr::from(8),
        ];

        let proof = prove(&mut circuit, &input.clone());

        // println!("Result: {:?}", proof);

        let input1 = [
            Fr::from(1),
            Fr::from(2),
            Fr::from(2),
            Fr::from(4),
            Fr::from(5),
            Fr::from(6),
            Fr::from(7),
            Fr::from(8),
        ];

        let verified = verify(proof, circuit, &input);
        println!("Verified: {:?}", verified);
        assert_eq!(verified, true);
    }
}

//
// Result:
// Proof {
//     output_poly: MultilinearPoly { polynomial: [938, 0] },
//     proof_polynomials: [
//             [
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 20392534301718573254799188749374153361497962288517655366201769616294618522877,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 8947344316974596338738173056591131585611030301357385521747675995960505101011,
//                             exp: 1
//                         },
//                         Term {
//                             coeff: 14436607124985380850955449684549265229987736210957027799446962760896493367346,
//                             exp: 0
//                         }
//                     ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 1461072887676773288743009273722258704590639561704498351353735634529550878004,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 3276843783591504766002395661300236744983160465410853157072447701687629655875,
//                             exp: 1
//                         }
//                     ],
//                     degree: 2
//                 }
//             ],
//             [
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 2043400694916232496964947044016014251764251031977583329975722958000552765319,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 1876406630826595457434032569802798103369024653330627786552931813380395603670,
//                             exp: 1
//                         },
//                         Term {
//                             coeff: 3843118017761700125602523371998987218074198524610508513461483381896194105296,
//                             exp: 0
//                         }
//                     ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 7689292708758583972308341024881457419811579983794923106591784330894416955684,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 16464351538782785350087872053052637555008338534972270746317491146102397674638,
//                             exp: 1
//                         },
//                         Term {
//                             coeff: 19622841496137181122096598412580455202276810282064874834487132896154802360912,
//                             exp: 0
//                         }
//                     ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 16076088174335052486351801648527176630500511051597480277906516082242139274967,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 5894136088811413150898981759105924560959820288910346806531750336929184945754,
//                             exp: 1
//                         }, Term {
//                             coeff: 12137895154004255554217839067815732081229171961415461537205652274910774782302,
//                             exp: 0
//                         }
//                     ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 17592265101642936948885160085544707605919397641519999346412218119444661417009,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 7320476945994771682520138252415976288113293645805385300690144583966467812865,
//                             exp: 1
//                         }
//                 ],
//                     degree: 2
//                 }
//             ],
//             [
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 9639892603938869200377086662386398276914092682999523012722086177818455617894,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 9706472777980028830786369660455593012997401756176853069273619512962874692123,
//                             exp: 1
//                         },
//                         Term {
//                             coeff: 14291356754607100488534692747969725627635046817979146094285991831301626627024,
//                             exp: 0
//                         }
//                 ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 19487237680903209671604550204755741656695491357371943510662488221165505990612,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 1245176504961613538606362309446144235358418292426865369269047732420002985627,
//                             exp: 1
//                         },
//                         Term {
//                             coeff: 127134263800813178029173910425856436750904648380903505928986323898520254922,
//                             exp: 0
//                         }
//                     ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 3891098896681817509083674698748483325159821852359067549912320264372127987483,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 5159159294039131490385690020763595352186316628193478864334659280046262229215,
//                             exp: 1
//                         },
//                         Term {
//                             coeff: 12837984681118326222777041025745196411202225919863487929451224642157418278919,
//                             exp: 0
//                         }
//                     ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 18316322801961913016638649301578729861863702496648830028599769370326300267362,
//                             exp: 2
//                                 },
//                         Term {
//                             coeff: 14902819671246711744918850280174503355412726370028823473255051088951360183117,
//                             exp: 1
//                         },
//                         Term {
//                             coeff: 12536700215478092449023874704115379047081205030317135953065762089442893173374,
//                             exp: 0
//                         }
//                     ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 7772834860586316383678666781926897102605532903988555595274149814442468499725,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 9461764726342563481760949523777745174687546734334619101909616895941235680930,
//                             exp: 1
//                         },
//                         Term {
//                             coeff: 9261699687236042274020436703855241327219982429001872599419618826562604205758,
//                             exp: 0
//                         }
//                     ],
//                     degree: 2
//                 },
//                 UnivariatePoly {
//                     co_ex: [
//                         Term {
//                             coeff: 17516436125990841383154427083316031694636335331360661136403881641159703664776,
//                             exp: 2
//                         },
//                         Term {
//                             coeff: 21055983605892951046040730063717924341558819347677912043660251655832305336455,
//                             exp: 1
//                         }
//                     ],
//                     degree: 2
//                 }
//             ]
//     ],
//     claimed_evaluations: [
//             (
//                 11504556113205068759693096395331238647544466924762229463476554927926964231098,
//                 13572130628097107895403491860733558278657290070433209341519472726473976664294
//             ),
//             (
//                 3603627254425994773455777801039310305249226891499986366557899534100095665650,
//                 421595593338255816901013955036972356653324994289757854301492762720824026280
//             )
//     ]
// }