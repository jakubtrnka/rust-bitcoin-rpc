//! Exact bitcoin amounts and fee rates.
//!
//! Bitcoin Core puts every BTC-denominated value on the wire as a JSON number
//! with up to eight decimals (`ValueFromAmount`, `src/core_write.cpp`), and
//! reads one back by parsing its literal decimal text, rejecting more than
//! eight decimals with `RPC_TYPE_ERROR`. Holding such a value in an `f64`
//! invites two mistakes: arithmetic on it accumulates binary rounding error
//! that Core then rejects, and comparing two of them for equality is
//! unreliable. [`Amount`], [`SignedAmount`] and [`FeeRate`] hold whole satoshis instead and
//! convert to and from Core's decimal form exactly.
//!
//! The conversion is exact because of the ranges involved: every amount
//! below [`Amount::MAX_MONEY`] has a distinct nearest `f64`, and the
//! shortest round-tripping decimal of that `f64` (what `serde_json` writes
//! and what `Display` for `f64` prints) is the original eight-decimal
//! value, so satoshis survive the trip through a JSON number unchanged.

use std::fmt;

/// Why an `f64` or a JSON number could not become an amount.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseAmountError {
    /// The value was NaN or infinite.
    NotFinite,
    /// The value was below zero for an unsigned amount.
    Negative,
    /// The value had more than eight decimal places, so it does not denote a
    /// whole number of satoshis. Typically a sign of `f64` arithmetic
    /// upstream (`0.1 + 0.2`); build the value in satoshis instead.
    TooPrecise,
    /// The absolute value exceeded 21 million BTC.
    TooLarge,
}

impl fmt::Display for ParseAmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseAmountError::NotFinite => write!(f, "amount is not a finite number"),
            ParseAmountError::Negative => write!(f, "amount is negative"),
            ParseAmountError::TooPrecise => {
                write!(f, "amount has more than 8 decimal places")
            }
            ParseAmountError::TooLarge => write!(f, "amount magnitude exceeds 21 million BTC"),
        }
    }
}

impl std::error::Error for ParseAmountError {}

/// A non-negative amount of bitcoin, held as whole satoshis.
///
/// Deserializes from and serializes to the BTC decimal number Bitcoin Core
/// uses on the wire, exactly (see the [module docs](self)). Construct one
/// from satoshis with [`Amount::from_sat`], or from a BTC `f64` with
/// [`Amount::from_btc`], which refuses anything that is not a whole number
/// of satoshis rather than rounding silently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(u64);

impl Amount {
    /// Zero satoshis.
    pub const ZERO: Amount = Amount(0);
    /// One satoshi.
    pub const ONE_SAT: Amount = Amount(1);
    /// One bitcoin, 100 000 000 satoshis.
    pub const ONE_BTC: Amount = Amount(100_000_000);
    /// The 21 million BTC supply cap, Core's `MAX_MONEY`. The largest value
    /// this type accepts from the wire.
    pub const MAX_MONEY: Amount = Amount(21_000_000 * 100_000_000);

    /// An amount of `sat` satoshis.
    pub const fn from_sat(sat: u64) -> Amount {
        Amount(sat)
    }

    /// The amount in satoshis.
    pub const fn to_sat(self) -> u64 {
        self.0
    }

    /// An amount from a BTC value, exactly.
    ///
    /// Fails unless `btc` is a finite, non-negative number of at most eight
    /// decimal places, no larger than [`Amount::MAX_MONEY`]. It never
    /// rounds: `Amount::from_btc(0.1 + 0.2)` is an error, not 30 000 000
    /// satoshis, because Core would reject `0.30000000000000004` too.
    pub fn from_btc(btc: f64) -> Result<Amount, ParseAmountError> {
        btc_to_sat(btc).map(Amount)
    }

    /// The amount in BTC, as the `f64` nearest to its exact decimal value.
    ///
    /// Suitable for display and for handing back to Core, which reads the
    /// shortest round-tripping decimal of this `f64` and gets the original
    /// satoshis back. Not suitable for arithmetic; do that in satoshis.
    pub fn to_btc(self) -> f64 {
        self.0 as f64 / 100_000_000.0
    }

    /// `self + rhs`, or `None` on overflow.
    pub const fn checked_add(self, rhs: Amount) -> Option<Amount> {
        match self.0.checked_add(rhs.0) {
            Some(sat) => Some(Amount(sat)),
            None => None,
        }
    }

    /// `self - rhs`, or `None` if `rhs` is the larger.
    pub const fn checked_sub(self, rhs: Amount) -> Option<Amount> {
        match self.0.checked_sub(rhs.0) {
            Some(sat) => Some(Amount(sat)),
            None => None,
        }
    }
}

impl fmt::Display for Amount {
    /// The BTC decimal form Core uses, always with eight decimals:
    /// `0.00012345`, `50.00000000`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:08}", self.0 / 100_000_000, self.0 % 100_000_000)
    }
}

/// A signed bitcoin amount, held as whole satoshis.
///
/// Used for mempool fees that include mining priority deltas and can therefore
/// be negative. The BTC conversion accepts magnitudes up to
/// [`Amount::MAX_MONEY`], with the same exact decimal conversion as [`Amount`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignedAmount(i64);

impl SignedAmount {
    /// Zero satoshis.
    pub const ZERO: SignedAmount = SignedAmount(0);

    /// An amount of `sat` satoshis.
    pub const fn from_sat(sat: i64) -> SignedAmount {
        SignedAmount(sat)
    }

    /// The amount in satoshis.
    pub const fn to_sat(self) -> i64 {
        self.0
    }

    /// An amount from BTC, rejecting non-finite values, fractional satoshis,
    /// and magnitudes above [`Amount::MAX_MONEY`].
    pub fn from_btc(btc: f64) -> Result<SignedAmount, ParseAmountError> {
        let magnitude = btc_to_sat(btc.abs())? as i64;
        Ok(SignedAmount(if btc.is_sign_negative() {
            -magnitude
        } else {
            magnitude
        }))
    }

    /// The nearest BTC `f64`; use satoshis for arithmetic.
    pub fn to_btc(self) -> f64 {
        self.0 as f64 / 100_000_000.0
    }
}

impl fmt::Display for SignedAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 < 0 {
            f.write_str("-")?;
        }
        let magnitude = self.0.unsigned_abs();
        write!(
            f,
            "{}.{:08}",
            magnitude / 100_000_000,
            magnitude % 100_000_000
        )
    }
}

/// A fee rate in satoshis per 1000 virtual bytes, Core's `CFeeRate`.
///
/// Core reports fee rates in BTC per kvB (`getmempoolinfo`,
/// `estimatesmartfee`, `getnetworkinfo`, ...) using the same decimal form as
/// amounts, and this type converts to and from that form exactly, like
/// [`Amount`]. `1 sat/vB` is `FeeRate::from_sat_per_kvb(1000)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeeRate(u64);

impl FeeRate {
    /// Zero.
    pub const ZERO: FeeRate = FeeRate(0);

    /// A rate of `sat` satoshis per 1000 virtual bytes.
    pub const fn from_sat_per_kvb(sat: u64) -> FeeRate {
        FeeRate(sat)
    }

    /// A rate of `sat` satoshis per virtual byte.
    pub const fn from_sat_per_vb(sat: u64) -> FeeRate {
        FeeRate(sat * 1000)
    }

    /// The rate in satoshis per 1000 virtual bytes.
    pub const fn to_sat_per_kvb(self) -> u64 {
        self.0
    }

    /// The rate in satoshis per virtual byte, which need not be whole.
    pub fn to_sat_per_vb(self) -> f64 {
        self.0 as f64 / 1000.0
    }

    /// A rate from a BTC/kvB value, exactly; see [`Amount::from_btc`] for
    /// what is refused.
    pub fn from_btc_per_kvb(btc: f64) -> Result<FeeRate, ParseAmountError> {
        btc_to_sat(btc).map(FeeRate)
    }

    /// The rate in BTC/kvB, as the `f64` nearest to its exact decimal value;
    /// see [`Amount::to_btc`].
    pub fn to_btc_per_kvb(self) -> f64 {
        self.0 as f64 / 100_000_000.0
    }

    /// The fee this rate charges for `vsize` virtual bytes, as Core computes
    /// it (`CFeeRate::GetFee`): truncating, but never zero for a non-zero
    /// rate and size, and `None` on overflow.
    pub fn fee_for_vsize(self, vsize: u64) -> Option<Amount> {
        let fee = self.0.checked_mul(vsize)? / 1000;
        Some(Amount(if fee == 0 && self.0 != 0 && vsize != 0 {
            1
        } else {
            fee
        }))
    }
}

impl fmt::Display for FeeRate {
    /// The BTC/kvB decimal form Core uses, with eight decimals.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:08}", self.0 / 100_000_000, self.0 % 100_000_000)
    }
}

/// The exact satoshi value of a BTC `f64`, via its shortest round-tripping
/// decimal text (what `Display` for `f64` prints; it never uses exponent
/// notation), so no binary rounding creeps in.
fn btc_to_sat(btc: f64) -> Result<u64, ParseAmountError> {
    if !btc.is_finite() {
        return Err(ParseAmountError::NotFinite);
    }
    if btc < 0.0 {
        return Err(ParseAmountError::Negative);
    }
    let text = btc.to_string();
    let (whole, frac) = text.split_once('.').unwrap_or((&text, ""));
    if frac.len() > 8 {
        return Err(ParseAmountError::TooPrecise);
    }
    // `-0.0` prints as "-0"; it is zero, not negative.
    let whole: u64 = whole
        .trim_start_matches('-')
        .parse()
        .map_err(|_| ParseAmountError::TooLarge)?;
    let frac: u64 = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<8}")
            .parse()
            .expect("at most 8 ASCII digits")
    };
    let sat = whole
        .checked_mul(100_000_000)
        .and_then(|s| s.checked_add(frac))
        .ok_or(ParseAmountError::TooLarge)?;
    if sat > Amount::MAX_MONEY.0 {
        return Err(ParseAmountError::TooLarge);
    }
    Ok(sat)
}

#[cfg(feature = "serde")]
mod serde_impls {
    use super::{Amount, FeeRate, SignedAmount, btc_to_sat};
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for Amount {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_f64(self.to_btc())
        }
    }

    impl<'de> Deserialize<'de> for Amount {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Amount, D::Error> {
            btc_to_sat(f64::deserialize(d)?)
                .map(Amount)
                .map_err(D::Error::custom)
        }
    }

    impl Serialize for SignedAmount {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_f64(self.to_btc())
        }
    }

    impl<'de> Deserialize<'de> for SignedAmount {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<SignedAmount, D::Error> {
            SignedAmount::from_btc(f64::deserialize(d)?).map_err(D::Error::custom)
        }
    }

    impl Serialize for FeeRate {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_f64(self.to_btc_per_kvb())
        }
    }

    impl<'de> Deserialize<'de> for FeeRate {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<FeeRate, D::Error> {
            btc_to_sat(f64::deserialize(d)?)
                .map(FeeRate)
                .map_err(D::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_btc_is_exact_for_core_style_decimals() {
        assert_eq!(Amount::from_btc(0.00012345).unwrap().to_sat(), 12_345);
        assert_eq!(Amount::from_btc(50.0).unwrap().to_sat(), 5_000_000_000);
        assert_eq!(Amount::from_btc(0.00000001).unwrap().to_sat(), 1);
        assert_eq!(Amount::from_btc(0.0).unwrap(), Amount::ZERO);
        assert_eq!(Amount::from_btc(-0.0).unwrap(), Amount::ZERO);
        assert_eq!(
            Amount::from_btc(20_999_999.99999999).unwrap().to_sat(),
            2_099_999_999_999_999
        );
        assert_eq!(Amount::from_btc(21_000_000.0).unwrap(), Amount::MAX_MONEY);
    }

    #[test]
    fn every_satoshi_count_round_trips_through_btc_at_the_edges() {
        // The argument in the module docs, checked where it is tightest: the
        // top of the range, where f64 spacing is coarsest, and the bottom.
        for sat in (Amount::MAX_MONEY.to_sat() - 10_000..=Amount::MAX_MONEY.to_sat())
            .chain(0..10_000)
            .chain([123_456_789, 1_234_567_890_123, 99_999_999_999_999])
        {
            let a = Amount::from_sat(sat);
            assert_eq!(Amount::from_btc(a.to_btc()), Ok(a), "{sat}");
        }
    }

    #[test]
    fn from_btc_refuses_what_core_would_refuse() {
        assert_eq!(
            Amount::from_btc(0.1 + 0.2),
            Err(ParseAmountError::TooPrecise)
        );
        assert_eq!(
            Amount::from_btc(0.000000001),
            Err(ParseAmountError::TooPrecise)
        );
        assert_eq!(Amount::from_btc(-1.0), Err(ParseAmountError::Negative));
        assert_eq!(Amount::from_btc(f64::NAN), Err(ParseAmountError::NotFinite));
        assert_eq!(
            Amount::from_btc(f64::INFINITY),
            Err(ParseAmountError::NotFinite)
        );
        assert_eq!(
            Amount::from_btc(21_000_000.00000001),
            Err(ParseAmountError::TooLarge)
        );
        assert_eq!(Amount::from_btc(1e21), Err(ParseAmountError::TooLarge));
    }

    #[test]
    fn display_is_cores_eight_decimal_form() {
        assert_eq!(Amount::from_sat(12_345).to_string(), "0.00012345");
        assert_eq!(Amount::from_sat(5_000_000_000).to_string(), "50.00000000");
        assert_eq!(Amount::ZERO.to_string(), "0.00000000");
        assert_eq!(FeeRate::from_sat_per_kvb(1_000).to_string(), "0.00001000");
    }

    #[test]
    fn signed_amount_conversion_and_display() {
        for sat in [
            -2_100_000_000_000_000,
            -12_345,
            -1,
            0,
            1,
            12_345,
            2_100_000_000_000_000,
        ] {
            let amount = SignedAmount::from_sat(sat);
            assert_eq!(SignedAmount::from_btc(amount.to_btc()), Ok(amount));
        }
        assert_eq!(SignedAmount::from_btc(-0.0), Ok(SignedAmount::ZERO));
        assert_eq!(SignedAmount::from_sat(-1).to_string(), "-0.00000001");
        assert_eq!(
            SignedAmount::from_sat(i64::MIN).to_string(),
            "-92233720368.54775808"
        );
        for btc in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                SignedAmount::from_btc(btc),
                Err(ParseAmountError::NotFinite)
            );
        }
        assert_eq!(
            SignedAmount::from_btc(-0.000000001),
            Err(ParseAmountError::TooPrecise)
        );
        assert_eq!(
            SignedAmount::from_btc(-21_000_001.0),
            Err(ParseAmountError::TooLarge)
        );
    }

    #[test]
    fn checked_arithmetic() {
        assert_eq!(
            Amount::ONE_BTC.checked_add(Amount::ONE_SAT),
            Some(Amount::from_sat(100_000_001))
        );
        assert_eq!(
            Amount::from_sat(u64::MAX).checked_add(Amount::ONE_SAT),
            None
        );
        assert_eq!(Amount::ONE_SAT.checked_sub(Amount::ONE_BTC), None);
        assert_eq!(
            Amount::ONE_BTC.checked_sub(Amount::ONE_BTC),
            Some(Amount::ZERO)
        );
    }

    #[test]
    fn fee_rate_conversions() {
        let one_sat_vb = FeeRate::from_sat_per_vb(1);
        assert_eq!(one_sat_vb.to_sat_per_kvb(), 1_000);
        assert_eq!(one_sat_vb.to_sat_per_vb(), 1.0);
        assert_eq!(one_sat_vb.to_btc_per_kvb(), 0.00001);
        assert_eq!(FeeRate::from_btc_per_kvb(0.00001).unwrap(), one_sat_vb);
        assert_eq!(FeeRate::from_sat_per_kvb(1_500).to_sat_per_vb(), 1.5);
    }

    #[test]
    fn fee_for_vsize_matches_cfeerate_getfee() {
        let rate = FeeRate::from_sat_per_kvb(1_000);
        assert_eq!(rate.fee_for_vsize(250), Some(Amount::from_sat(250)));
        // Truncates ...
        assert_eq!(
            FeeRate::from_sat_per_kvb(1_001).fee_for_vsize(250),
            Some(Amount::from_sat(250))
        );
        // ... but never to zero for a non-zero rate and size.
        assert_eq!(
            FeeRate::from_sat_per_kvb(1).fee_for_vsize(1),
            Some(Amount::ONE_SAT)
        );
        assert_eq!(FeeRate::ZERO.fee_for_vsize(250), Some(Amount::ZERO));
        assert_eq!(rate.fee_for_vsize(0), Some(Amount::ZERO));
        assert_eq!(FeeRate::from_sat_per_kvb(u64::MAX).fee_for_vsize(2), None);
    }

    #[cfg(feature = "serde")]
    mod serde {
        use super::*;
        use serde_json::json;

        #[test]
        fn deserializes_cores_wire_form() {
            assert_eq!(
                serde_json::from_value::<Amount>(json!(0.00012345)).unwrap(),
                Amount::from_sat(12_345)
            );
            // Core writes whole BTC as "50.00000000", which serde_json reads
            // as 50.0; a bare integer must work too.
            assert_eq!(
                serde_json::from_value::<Amount>(json!(50)).unwrap(),
                Amount::from_sat(5_000_000_000)
            );
            assert_eq!(
                serde_json::from_str::<Amount>("50.00000000").unwrap(),
                Amount::from_sat(5_000_000_000)
            );
            assert_eq!(
                serde_json::from_value::<FeeRate>(json!(0.00001000)).unwrap(),
                FeeRate::from_sat_per_kvb(1_000)
            );
        }

        #[test]
        fn rejects_imprecise_negative_and_non_numeric() {
            assert!(serde_json::from_value::<Amount>(json!(0.123456789)).is_err());
            assert!(serde_json::from_value::<Amount>(json!(-1.0)).is_err());
            assert!(serde_json::from_value::<Amount>(json!("0.5")).is_err());
            assert!(serde_json::from_value::<Amount>(json!(null)).is_err());
        }

        #[test]
        fn serializes_to_the_shortest_decimal_core_accepts() {
            assert_eq!(
                serde_json::to_string(&Amount::from_sat(12_345)).unwrap(),
                "0.00012345"
            );
            assert_eq!(
                serde_json::to_string(&Amount::from_sat(5_000_000_000)).unwrap(),
                "50.0"
            );
            assert_eq!(serde_json::to_string(&Amount::ZERO).unwrap(), "0.0");
            assert_eq!(
                serde_json::to_string(&Amount::MAX_MONEY).unwrap(),
                "21000000.0"
            );
            assert_eq!(
                serde_json::to_string(&FeeRate::from_sat_per_kvb(1_000)).unwrap(),
                "0.00001"
            );
        }

        #[test]
        fn signed_wire_round_trip_is_lossless_at_the_edges() {
            for magnitude in
                (Amount::MAX_MONEY.to_sat() - 1_000..=Amount::MAX_MONEY.to_sat()).chain(0..1_000)
            {
                for sign in [-1, 1] {
                    let amount = SignedAmount::from_sat(sign * magnitude as i64);
                    let wire = serde_json::to_string(&amount).unwrap();
                    assert_eq!(serde_json::from_str::<SignedAmount>(&wire).unwrap(), amount);
                }
            }
            assert!(serde_json::from_str::<SignedAmount>("-0.000000001").is_err());
            assert!(serde_json::from_str::<SignedAmount>(r#""-0.1""#).is_err());
        }

        #[test]
        fn wire_round_trip_is_lossless_at_the_edges() {
            for sat in
                (Amount::MAX_MONEY.to_sat() - 1_000..=Amount::MAX_MONEY.to_sat()).chain(0..1_000)
            {
                let a = Amount::from_sat(sat);
                let text = serde_json::to_string(&a).unwrap();
                assert_eq!(serde_json::from_str::<Amount>(&text).unwrap(), a, "{sat}");
            }
        }
    }
}
