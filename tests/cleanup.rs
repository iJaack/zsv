use zsv::{Action, Asset, Network, Pubkey, SignedOracleReport, SignedTx};

#[test]
fn transaction_signing_bytes_preserve_every_action_layout() {
    let tx = SignedTx {
        network: Network::Testnet,
        chain_domain: [b'd'; 32],
        signer: Pubkey([b's'; 32]),
        nonce: 0x0807_0605_0403_0201,
        actions: vec![
            Action::Open {
                vault_id: 1,
                collateral: 0x0807_0605_0403_0201,
            },
            Action::Deposit {
                vault_id: 2,
                amount: u64::MAX,
            },
            Action::Borrow {
                vault_id: 3,
                amount: 0,
            },
            Action::Repay {
                vault_id: 4,
                amount: 1,
            },
            Action::Withdraw {
                vault_id: 5,
                amount: 2,
            },
            Action::Close { vault_id: 6 },
            Action::Liquidate {
                vault_id: 7,
                owner: Pubkey([b'o'; 32]),
                oracle_sequence: 8,
                max_burn: 9,
                min_collateral: 10,
            },
            Action::Transfer {
                asset: Asset::ZEC,
                to: Pubkey([b't'; 32]),
                amount: 11,
            },
            Action::Transfer {
                asset: Asset::ZUSD,
                to: Pubkey([b'u'; 32]),
                amount: 12,
            },
        ],
        signature: vec![],
    };
    // Literal wire bytes protect tags, field order and widths independently of
    // the encoder. Zero and maximum amounts remain encodable before validation.
    let expected: &[&[u8]] = &[
        b"zsv/tx/v2\0\x02",
        b"dddddddddddddddddddddddddddddddd",
        b"ssssssssssssssssssssssssssssssss",
        b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\0\0\0\0\0\0\0",
        b"\x00\x01\0\0\0\0\0\0\0\x01\x02\x03\x04\x05\x06\x07\x08", // Open
        b"\x01\x02\0\0\0\0\0\0\0\xff\xff\xff\xff\xff\xff\xff\xff", // Deposit
        b"\x02\x03\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",                 // Borrow
        b"\x03\x04\0\0\0\0\0\0\0\x01\0\0\0\0\0\0\0",               // Repay
        b"\x04\x05\0\0\0\0\0\0\0\x02\0\0\0\0\0\0\0",               // Withdraw
        b"\x05\x06\0\0\0\0\0\0\0",                                 // Close
        b"\x06\x07\0\0\0\0\0\0\0",                                 // Liquidate
        b"oooooooooooooooooooooooooooooooo",
        b"\x08\0\0\0\0\0\0\0\x09\0\0\0\0\0\0\0\x0a\0\0\0\0\0\0\0",
        b"\x07\x00tttttttttttttttttttttttttttttttt\x0b\0\0\0\0\0\0\0", // ZEC transfer
        b"\x07\x01uuuuuuuuuuuuuuuuuuuuuuuuuuuuuuuu\x0c\0\0\0\0\0\0\0", // ZUSD transfer
    ];
    assert_eq!(tx.signing_bytes(), expected.concat());
}

#[test]
fn oracle_signing_bytes_preserve_header_and_numeric_field_order() {
    let report = SignedOracleReport {
        network: Network::Mainnet,
        chain_domain: [b'd'; 32],
        signer: Pubkey([b's'; 32]),
        sequence: 0x0807_0605_0403_0201,
        price: u64::MAX,
        valid_until: 0,
        signature: vec![],
    };
    let expected: &[&[u8]] = &[
        b"zsv/oracle/v2\0\x01",
        b"dddddddddddddddddddddddddddddddd",
        b"ssssssssssssssssssssssssssssssss",
        b"\x01\x02\x03\x04\x05\x06\x07\x08",
        b"\xff\xff\xff\xff\xff\xff\xff\xff",
        b"\0\0\0\0\0\0\0\0",
    ];
    assert_eq!(report.signing_bytes(), expected.concat());
}
