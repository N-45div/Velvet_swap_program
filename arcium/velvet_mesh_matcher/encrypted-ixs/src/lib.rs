use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    pub const NO_VALID_QUOTE: u8 = 255;

    pub struct QuoteInput {
        output_amount: u64,
        risk_bps: u16,
        route: u8,
    }

    pub struct PrivateMatchInput {
        min_output_amount: u64,
        max_risk_bps: u16,
        preferred_route: u8,
        quote_0: QuoteInput,
        quote_1: QuoteInput,
        quote_2: QuoteInput,
    }

    #[instruction]
    pub fn select_private_quote(input_ctxt: Enc<Shared, PrivateMatchInput>) -> u8 {
        let input = input_ctxt.to_arcis();
        let mut best_index = NO_VALID_QUOTE;
        let mut best_output = 0u64;

        let quote_0_valid = input.quote_0.output_amount >= input.min_output_amount
            && input.quote_0.risk_bps <= input.max_risk_bps
            && input.quote_0.route == input.preferred_route;
        if quote_0_valid {
            best_index = 0;
            best_output = input.quote_0.output_amount;
        }

        let quote_1_valid = input.quote_1.output_amount >= input.min_output_amount
            && input.quote_1.risk_bps <= input.max_risk_bps
            && input.quote_1.route == input.preferred_route;
        if quote_1_valid && input.quote_1.output_amount > best_output {
            best_index = 1;
            best_output = input.quote_1.output_amount;
        }

        let quote_2_valid = input.quote_2.output_amount >= input.min_output_amount
            && input.quote_2.risk_bps <= input.max_risk_bps
            && input.quote_2.route == input.preferred_route;
        if quote_2_valid && input.quote_2.output_amount > best_output {
            best_index = 2;
        }

        best_index.reveal()
    }
}
