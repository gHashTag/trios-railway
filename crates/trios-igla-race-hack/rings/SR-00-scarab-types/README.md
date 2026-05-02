# SR-00: Scarab Types

Break down the 27 scarab types from NEON into 5 atomic rings.

## Rings

| Ring | Scarab Types | Total |
|------|--------------|-------|
| O-Type | Ⲁⲁ (0,1) | 2 |
| One-Type | Ⲃⲃ (0-1,0) | 5 |
| Two-Type | ⲁⲃ (2) | 2 |
| Three-Type | ⲃⲄ (0-1,0) | 4 |
| Four-Type | ⲁⲃ (2) | 2 |
| Five-Type | ⲀⲄ (3) | 2 |
| Six-Type | Ⲅ (2) | 2 |
| Seven-Type | ⲁⲃ (3) | 2 |
| Eight-Type | ⲀⲄ (2) | 2 |
| Nine-Type | Ⲁⲁ (2) | 2 |
| **Total** | **27** | |

## Usage

```rust
use sr_00_scarab_types::{ScarabType, RingCategory};

// Get all O-type scarab types
let o_types = ScarabType::OType.codes();
println!("O-Types: {:?}", o_types);

// Get total scarab types in a category
let count = ScarabType::OneType.count();
println!("One-Type has {} scarab types", count);
```

## Acceptance

- Each ring exports `ScarabType` enum
- `codes()` returns all scarab type codes in that category
- `count()` returns total scarab types in that category
- Each category implements `Display` with `as_display()`
- README.md explains the scarab type taxonomy

**Part of #446** — Ring-Pattern Refactor (3 GOLD ~19 SR)
