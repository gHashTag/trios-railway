//! Canonical Railway account registry (7 accounts per operator instruction 2026-05-02).
//!
//! Source of truth, do not hardcode elsewhere. Each record is a compile-time constant;
//! the token is read from the corresponding `RAILWAY_TOKEN_ACC{0..6}` env at runtime
//! to avoid leaking secrets into the binary.
//!
//! Lane→account mapping follows trios#445.

/// One of the 7 operator-supplied Railway accounts.
pub struct Account {
    pub tag: &'static str,
    pub env_tok: &'static str,
    pub project: &'static str,
    pub environment: &'static str,
    pub kind: TokenKind,
    pub lane: &'static str,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Project,
    Personal,
}

impl TokenKind {
    pub fn auth_header(self, tok: &str) -> (&'static str, String) {
        match self {
            TokenKind::Project => ("Project-Access-Token", tok.into()),
            TokenKind::Personal => ("Authorization", format!("Bearer {tok}")),
        }
    }
}

pub const ACCOUNTS: &[Account] = &[
    Account {
        tag: "acc0",
        env_tok: "RAILWAY_TOKEN_ACC0",
        project: "f29aa9dd-ca0b-460f-ad24-c7680c6717fb",
        environment: "fade0d77-af80-4d01-bc34-2ce27283d766",
        kind: TokenKind::Project,
        lane: "IGLA-RAILWAY-FOLLOWER-A",
    },
    Account {
        tag: "acc1",
        env_tok: "RAILWAY_TOKEN_ACC1",
        project: "e4fe33bb-3b09-4842-9782-7d2dea1abc9b",
        environment: "54e293b9-00a9-4102-814d-db151636d96e",
        kind: TokenKind::Personal,
        lane: "IGLA-RAILWAY-LEADER",
    },
    Account {
        tag: "acc2",
        env_tok: "RAILWAY_TOKEN_ACC2",
        project: "12c508c7-1196-468d-b06d-d8de8cb77e93",
        environment: "441bd3a6-f6d8-455e-b567-376b7538e9f1",
        kind: TokenKind::Personal,
        lane: "IGLA-RAILWAY-FOLLOWER-B",
    },
    Account {
        tag: "acc3",
        env_tok: "RAILWAY_TOKEN_ACC3",
        project: "8ab06401-aa28-4af7-9faf-39a1548b7008",
        environment: "cd2d987b-dbbb-49ba-953b-f5e9486b906c",
        kind: TokenKind::Personal,
        lane: "IGLA-RAILWAY-FOLLOWER-C",
    },
    Account {
        tag: "acc4",
        env_tok: "RAILWAY_TOKEN_ACC4",
        project: "0247abaa-6487-4347-811c-168d7fe53078",
        environment: "336c41a9-0d6a-4308-b266-1df6c91590ac",
        kind: TokenKind::Personal,
        lane: "IGLA-RAILWAY-FOLLOWER-D",
    },
    Account {
        tag: "acc5",
        env_tok: "RAILWAY_TOKEN_ACC5",
        project: "475a2290-d990-426a-af57-594a934cf6f4",
        environment: "5724292a-1c7d-42ca-8859-edcab337c5a9",
        kind: TokenKind::Project,
        lane: "IGLA-RAILWAY-FOLLOWER-E",
    },
    Account {
        tag: "acc6",
        env_tok: "RAILWAY_TOKEN_ACC6",
        project: "475a2290-d990-426a-af57-594a934cf6f4",
        environment: "5724292a-1c7d-42ca-8859-edcab337c5a9",
        kind: TokenKind::Project,
        lane: "IGLA-RAILWAY-SPRINT-X",
    },
];

/// Sanctioned seeds (quorum) per `enforce_seed_policy()` trigger in Neon.
/// Fibonacci F17..F21. Never queue a `priority=0` row with a seed not in this set.
pub const SANCTIONED_SEEDS: &[u64] = &[1597, 2584, 4181, 6765, 10946];

/// Quick-3 Fibonacci (smaller) used for phi-LR ladder Quick-3 gate.
pub const QUICK3_SEEDS: &[u64] = &[34, 55, 89];
