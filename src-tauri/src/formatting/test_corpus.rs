//! 500-case formatting corpus.
//!
//! Each category lives in its own `#[test]` so failures are isolated. Tests
//! collect ALL failures within a category and report them in one panic
//! message — no early bailout — so a single `cargo test` run gives the
//! complete landscape of what works and what doesn't.
//!
//! Expected outputs reflect the *desired* behaviour, not necessarily what
//! today's pipeline produces. Failing cases are useful — they tell us
//! exactly where the formatter falls short.

#![cfg(test)]

use crate::formatting::{format, FormattingConfig, FormattingContext, FormattingMode};

fn cfg(mode: FormattingMode) -> FormattingConfig {
    FormattingConfig {
        enabled: true,
        mode,
        custom_fillers: Vec::new(),
        detect_app_context: false,
        user_full_name: String::new(),
    }
}

/// Run a corpus and report ALL mismatches in one panic message.
fn run_corpus(name: &str, cases: &[(&str, &str)], mode: FormattingMode) {
    let mut failures = Vec::new();
    let ctx = FormattingContext::default();
    let c = cfg(mode);
    for (i, (input, expected)) in cases.iter().enumerate() {
        let actual = format(input, &c, &ctx);
        if actual.trim() != expected.trim() {
            failures.push(format!(
                "  [{:>3}] input:    {:?}\n        expected: {:?}\n        actual:   {:?}",
                i + 1,
                input,
                expected,
                actual
            ));
        }
    }
    let total = cases.len();
    let passed = total - failures.len();
    if !failures.is_empty() {
        panic!(
            "\n=== {} ===\n{} of {} passed ({} failed)\n\n{}\n",
            name,
            passed,
            total,
            failures.len(),
            failures.join("\n\n")
        );
    }
}

// ============================================================
// 1. FILLERS — 50 cases
// ============================================================

#[test]
fn corpus_01_fillers() {
    let cases: &[(&str, &str)] = &[
        // Basic standalone fillers
        ("um hello world", "Hello world."),
        ("uh hello world", "Hello world."),
        ("hmm hello world", "Hello world."),
        ("er hello world", "Hello world."),
        ("erm hello world", "Hello world."),
        ("ah hello world", "Hello world."),
        // At end
        ("hello world um", "Hello world."),
        ("hello world uh", "Hello world."),
        // Multiple fillers
        ("um uh hello hmm world", "Hello world."),
        ("um, hello, uh, world", "Hello, world."),
        // Comma-flanked
        ("I, um, want pizza", "I want pizza."),
        ("I, uh, need help", "I need help."),
        ("we, hmm, should think", "We should think."),
        // Parenthetical "like" (filler)
        ("I, like, want pizza", "I want pizza."),
        ("I was, like, so confused", "I was so confused."),
        // "Like" preserved as verb
        ("I like pizza", "I like pizza."),
        ("we like the new design", "We like the new design."),
        ("she likes coffee", "She likes coffee."),
        // "Basically" parenthetical
        ("I, basically, agree", "I agree."),
        ("we, basically, finished", "We finished."),
        // "Basically" preserved when meaningful
        ("the basically same approach", "The basically same approach."),
        // "Actually"
        ("I, actually, agree", "I agree."),
        ("actually, I changed my mind", "I changed my mind."),
        // "You know"
        ("the thing is, you know, complex", "The thing is complex."),
        ("we should, you know, ship it", "We should ship it."),
        // "I mean"
        ("the project is, I mean, late", "The project is late."),
        // Mixed clean
        ("um so like I was thinking we should ship", "So I was thinking we should ship."),
        // Repeated word collapse
        ("the the cat sat down", "The cat sat down."),
        ("we we should should ship", "We should ship."),
        ("hello hello world", "Hello world."),
        // Short repeats preserved (intentional emphasis)
        ("no no please", "No no please."),
        ("very very fast", "Very very fast."),
        // Filler at start with comma
        ("um, hello there", "Hello there."),
        ("uh, what time is it", "What time is it."),
        // Multiple commas
        ("well, um, you know, I think we should", "Well, I think we should."),
        // No fillers — passthrough
        ("the quick brown fox", "The quick brown fox."),
        ("she went to the store", "She went to the store."),
        // Already capitalised + punctuated
        ("Hello world.", "Hello world."),
        ("How are you?", "How are you?"),
        // Filler within punctuation
        ("hello. um, world.", "Hello. World."),
        ("first thing. uh, second thing.", "First thing. Second thing."),
        // Short utterances
        ("um yes", "Yes."),
        ("uh ok", "Ok."),
        ("hmm", ""),
        // Tricky: filler-looking real words
        ("I read the article", "I read the article."),
        ("the umbrella is broken", "The umbrella is broken."),
        ("the urinary tract", "The urinary tract."),
        // Multiple fillers same sentence
        ("um like basically you know I think", "I think."),
        // Filler with punctuation right after
        ("um. hello", "Hello."),
        ("uh? hello", "Hello."),
        // Edge: only fillers
        ("um uh hmm", ""),
        // Trailing punctuation preservation
        ("um, hello, world!", "Hello, world!"),
    ];
    run_corpus("01-fillers", cases, FormattingMode::Smart);
}

// ============================================================
// 2. SELF-CORRECTIONS — 40 cases
// ============================================================

#[test]
fn corpus_02_corrections() {
    let cases: &[(&str, &str)] = &[
        // Inline "no wait"
        ("go to the office no wait go to the cafe", "Go to the cafe."),
        ("call John no wait call Sarah", "Call Sarah."),
        ("send 100 dollars no wait send 200 dollars", "Send 200 dollars."),
        // Sentence-spanning "no wait"
        ("Let's meet at the office. No wait, let's meet at the cafe.", "Let's meet at the cafe."),
        ("I'll call John. No wait, I'll text him.", "I'll text him."),
        // "Sorry"
        ("my name is Raj sorry Rajesh", "My name is Rajesh."),
        ("she lives in Mumbai sorry Bangalore", "She lives in Bangalore."),
        // "I mean"
        ("the deadline is Tuesday I mean Wednesday", "The deadline is Wednesday."),
        ("we need 5 I mean 10 servers", "We need 10 servers."),
        // "Scratch that"
        ("send the report scratch that send the proposal", "Send the proposal."),
        // "Actually" as correction
        ("the meeting is at 3 actually it's at 4", "It's at 4."),
        // No correction marker — passthrough
        ("just a regular sentence", "Just a regular sentence."),
        ("no problems here", "No problems here."),
        // "No" without "wait" — should NOT trigger correction
        ("no problem with that", "No problem with that."),
        ("no it's fine", "No it's fine."),
        // Comma-flanked "no, wait,"
        ("send John an email, no, wait, send Sarah", "Send Sarah."),
        // Whisper-style with embedded comma in marker
        ("call mom no, wait call dad", "Call dad."),
        ("the price is 100 no, wait, 200", "200."),
        // Multiple corrections (chained)
        ("call John no wait call Sarah no wait call Mary", "Call Mary."),
        // Correction at very start
        ("no wait actually I'll do it later", "Actually I'll do it later."),
        // Correction with email body preserved
        (
            "Email to Raj saying meeting at 5 no wait, meeting at 6",
            "Hi Raj,\n\nMeeting at 6.\n\nBest regards,",
        ),
        // Correction inside a list
        ("first call John no wait call Sarah second send the email", "First call Sarah second send the email."),
        // Real-world sentence with "wait" but no correction
        ("I had to wait for the bus", "I had to wait for the bus."),
        ("wait until tomorrow", "Wait until tomorrow."),
        // "I mean" as parenthetical, not correction
        ("the thing is, I mean, complicated", "The thing is complicated."),
        // Period before "no wait" (Whisper auto-period)
        ("send 100 dollars. No wait, send 200 dollars.", "Send 200 dollars."),
        // Triple correction marker — keep last
        ("apple sorry banana sorry cherry", "Cherry."),
        // Correction without space around marker
        ("100 no wait 200", "200."),
        // Correction in mixed case
        ("Send John SORRY Send Sarah", "Send Sarah."),
        // Long prior clause — sentence boundary version
        (
            "We all agreed to meet at the office at 5pm. No wait, we said the cafe at 6.",
            "We said the cafe at 6.",
        ),
        // Correction with question
        ("is the meeting today no wait is it tomorrow", "Is it tomorrow."),
        // Correction at end (just delete prior — no replacement)
        ("send 100 dollars no wait", "Send 100 dollars no wait."),
        // Spurious "no wait" without anything after
        ("the answer is yes no wait", "The answer is yes no wait."),
        // Correction inside list-style utterance
        ("first apple second banana sorry orange third grape", "First apple second orange third grape."),
        // Sentence-boundary with !
        ("Send him! No wait, send her!", "Send her!"),
        // Sentence-boundary with ?
        ("Should we go? No wait, let's stay.", "Let's stay."),
        // Interpolated "actually" that IS a correction
        ("call John, actually, call Sarah", "Call Sarah."),
        // Interpolated "actually" that ISN'T (no replacement)
        ("call John, actually", "Call John."),
        // Repeated "sorry sorry"
        ("send Raj sorry sorry send Priya", "Send Priya."),
    ];
    run_corpus("02-corrections", cases, FormattingMode::Smart);
}

// ============================================================
// 3. SPOKEN PUNCTUATION — 60 cases
// ============================================================

#[test]
fn corpus_03_punctuation() {
    let cases: &[(&str, &str)] = &[
        // Basic comma
        ("hello comma world", "Hello, world."),
        ("yes comma I agree", "Yes, I agree."),
        // Full stop / period
        ("hello world full stop", "Hello world."),
        ("the end period", "The end."),
        // Question mark
        ("how are you question mark", "How are you?"),
        ("are you sure question mark", "Are you sure?"),
        // Exclamation
        ("amazing exclamation mark", "Amazing!"),
        ("hooray exclamation point", "Hooray!"),
        // Colon
        ("warning colon do not press", "Warning: do not press."),
        ("note colon this is important", "Note: this is important."),
        // Semicolon
        ("done semicolon moving on", "Done; moving on."),
        // Dash
        ("breaking news dash big news", "Breaking news — big news."),
        // New line
        ("line one new line line two", "Line one\nLine two."),
        ("first new line second", "First\nSecond."),
        // New paragraph
        ("paragraph one new paragraph paragraph two", "Paragraph one\n\nParagraph two."),
        // Quotes
        ("he said open quote hello close quote", "He said \"hello\"."),
        // Parens
        ("the answer open paren maybe close paren is yes", "The answer (maybe) is yes."),
        // Multiple punctuation
        (
            "hello comma how are you question mark fine exclamation mark",
            "Hello, how are you? Fine!",
        ),
        // Punctuation as noun (should NOT convert)
        ("I had a great period in life", "I had a great period in life."),
        ("the colon is part of digestion", "The colon is part of digestion."),
        ("the comma key is broken", "The comma key is broken."),
        ("a long dash is em dash", "A long dash is em dash."),
        // Already-punctuated input
        ("Hello, world.", "Hello, world."),
        ("Yes! No?", "Yes! No?"),
        // No spoken punctuation — let auto-punct work
        ("just a regular thought", "Just a regular thought."),
        // End-of-sentence question mark
        ("are we ready question mark", "Are we ready?"),
        // Interleaved
        (
            "first thing comma second thing comma third thing full stop",
            "First thing, second thing, third thing.",
        ),
        // New line inside sentence
        ("dear John new line how are you", "Dear John\nHow are you."),
        // Multiple new paragraphs
        (
            "intro new paragraph body new paragraph conclusion",
            "Intro\n\nBody\n\nConclusion.",
        ),
        // Comma at end
        ("thanks comma", "Thanks,"),
        // Period at end (already there)
        ("done.", "Done."),
        // Spoken parens
        (
            "the result open parenthesis surprising close parenthesis was good",
            "The result (surprising) was good.",
        ),
        // Quote with period
        ("she said open quote hi close quote period", "She said \"hi\"."),
        // Tricky: punctuation word at the end of sentence
        ("I prefer the comma over the semicolon", "I prefer the comma over the semicolon."),
        // Multiple spoken commas
        (
            "apples comma bananas comma cherries comma grapes",
            "Apples, bananas, cherries, grapes.",
        ),
        // Dash mid-sentence
        ("the answer dash maybe dash is yes", "The answer — maybe — is yes."),
        // Trailing question mark with extra "uh"
        ("uh how are you question mark", "How are you?"),
        // Period right after a number
        ("the year was 2023 period", "The year was 2023."),
        // Punctuation in proper name (don't convert)
        ("call John Smith", "Call John Smith."),
        // "Question" as a noun (not punctuation)
        ("I have a question for you", "I have a question for you."),
        ("the question mark is at the end", "The question mark is at the end."),
        // "Period" as noun (already covered) — repeat for emphasis
        ("the third period of class", "The third period of class."),
        // Edge: just punctuation words
        ("comma", "Comma."),
        ("question mark", "Question mark."),
        // Sentence with both new line and full stop
        ("first thing new line second thing full stop", "First thing\nSecond thing."),
        // Combined: comma + question mark
        ("hello comma how are you question mark", "Hello, how are you?"),
        // Combined: comma + exclamation
        ("wow comma that's amazing exclamation mark", "Wow, that's amazing!"),
        // Mixed case spoken cmd
        ("Hello COMMA world", "Hello, world."),
        // Spoken with leading uppercase
        ("Hello Comma World", "Hello, world."),
        // Period at sentence start (no-op)
        ("period is a good word", "Period is a good word."),
        // Multiple periods
        ("done period done period done period", "Done. Done. Done."),
        // Half-finished spoken cmd
        ("hello comma", "Hello,"),
        // Spurious extra spaces
        ("hello   comma   world", "Hello, world."),
        // Quote within quote
        (
            "he said open quote she said open quote hi close quote close quote",
            "He said \"she said \"hi\"\".",
        ),
        // Common abbreviations should not break
        ("Dr. Smith called", "Dr. Smith called."),
        ("U.S.A. is large", "U.S.A. is large."),
        // Currency symbols already in text
        ("the price is $100", "The price is $100."),
        // Complex: list with spoken commas
        (
            "items are colon apples comma oranges comma bananas",
            "Items are: apples, oranges, bananas.",
        ),
    ];
    run_corpus("03-punctuation", cases, FormattingMode::Smart);
}

// ============================================================
// 4. NUMBERS — 50 cases
// ============================================================

#[test]
fn corpus_04_numbers() {
    let cases: &[(&str, &str)] = &[
        // Cardinals
        ("I have one apple", "I have 1 apple."),
        ("she has two cats", "She has 2 cats."),
        ("there are five people", "There are 5 people."),
        ("ten years old", "10 years old."),
        ("twelve months", "12 months."),
        ("twenty employees", "20 employees."),
        // Compound cardinals
        ("twenty five apples", "25 apples."),
        ("thirty seven kilometres", "37 kilometres."),
        ("one hundred dollars", "100 dollars."),
        ("two hundred fifty pages", "250 pages."),
        ("five hundred", "500."),
        ("nine hundred ninety nine", "999."),
        // Thousands
        ("one thousand", "1000."),
        ("two thousand five hundred", "2500."),
        ("twenty five thousand", "25000."),
        ("one hundred thousand", "100000."),
        // Indian magnitudes
        ("one lakh", "100000."),
        ("five lakh", "500000."),
        ("one crore", "10000000."),
        ("two crore", "20000000."),
        ("ten lakh fifty thousand", "1050000."),
        // Hyphenated
        ("twenty-five", "25."),
        ("thirty-seven", "37."),
        ("ninety-nine", "99."),
        // Mixed digit + word (KNOWN BUG: "50 thousand" → "50 1000")
        ("we sold 50 thousand units", "We sold 50000 units."),
        ("about 25 million people", "About 25000000 people."),
        // Decimals
        ("five point five", "5.5."),
        ("three point one four", "3.14."),
        ("zero point five", "0.5."),
        // Ordinals
        ("the first place", "The 1st place."),
        ("the second time", "The 2nd time."),
        ("the third option", "The 3rd option."),
        ("the twenty fifth anniversary", "The 25th anniversary."),
        // Ranges
        ("from five to ten", "From 5 to 10."),
        ("between one hundred and two hundred", "Between 100 and 200."),
        // Phone-number-ish
        ("call nine one one", "Call 911."),
        ("dial five five five one two three four", "Dial 5551234."),
        // Years
        ("born in nineteen ninety", "Born in 1990."),
        ("the year two thousand twenty four", "The year 2024."),
        // Negative
        ("minus five degrees", "-5 degrees."),
        ("temperature dropped to minus ten", "Temperature dropped to -10."),
        // Fractions
        ("one half", "1/2."),
        ("three quarters", "3/4."),
        ("two thirds", "2/3."),
        // Mixed numbers
        ("one and a half", "1.5."),
        ("two and three quarters", "2.75."),
        // Already-digit input — pass through
        ("there are 5 cats", "There are 5 cats."),
        ("call 911 immediately", "Call 911 immediately."),
        // Number words used non-numerically
        ("the One True Path", "The One True Path."),
        ("a million reasons", "A million reasons."),
    ];
    run_corpus("04-numbers", cases, FormattingMode::Smart);
}

// ============================================================
// 5. CURRENCY — 50 cases
// ============================================================

#[test]
fn corpus_05_currency() {
    let cases: &[(&str, &str)] = &[
        // INR rupees
        ("send five hundred rupees", "Send ₹500."),
        ("transfer 1000 rupees", "Transfer ₹1,000."),
        ("twenty five thousand rupees", "₹25,000."),
        ("one lakh rupees", "₹1,00,000."),
        ("two crore rupees", "₹2,00,00,000."),
        ("ten lakh rupees", "₹10,00,000."),
        ("Rs 250", "₹250."),
        ("Rs. 500", "₹500."),
        ("INR 1000", "₹1,000."),
        ("100 INR", "₹100."),
        // USD
        ("five dollars", "$5."),
        ("twenty dollars", "$20."),
        ("one hundred dollars", "$100."),
        ("two thousand dollars", "$2,000."),
        ("$50", "$50."),
        ("USD 500", "$500."),
        // EUR
        ("ten euros", "€10."),
        ("fifty euros", "€50."),
        ("one hundred euros", "€100."),
        ("EUR 250", "€250."),
        // GBP
        ("five pounds", "£5."),
        ("fifty pounds", "£50."),
        ("two hundred pounds", "£200."),
        ("GBP 100", "£100."),
        // Mixed currency in one sentence
        (
            "I owe you 50 dollars and she owes 100 rupees",
            "I owe you $50 and she owes ₹100.",
        ),
        // Already symbol
        ("the price is $100", "The price is $100."),
        ("worth €500", "Worth €500."),
        // Currency-like words preserved
        ("the dollar bill in my pocket", "The dollar bill in my pocket."),
        ("euros and pounds are currencies", "Euros and pounds are currencies."),
        // Percentages
        ("five percent", "5%."),
        ("five point five percent", "5.5%."),
        ("twenty five percent", "25%."),
        ("hundred percent agree", "100% agree."),
        // Currency with decimals
        ("ten dollars fifty cents", "$10.50."),
        ("five point five dollars", "$5.50."),
        // Currency in ranges
        ("between 100 and 200 dollars", "Between $100 and $200."),
        ("five to ten lakh rupees", "₹5,00,000 to ₹10,00,000."),
        // Crore/lakh formatting
        ("1.5 crore rupees", "₹1,50,00,000."),
        ("2.5 lakh rupees", "₹2,50,000."),
        // Currency in question
        ("how much is 100 dollars in rupees question mark", "How much is $100 in ₹?"),
        // Misheard rupee variants
        ("send 500 rupes", "Send 500 rupes."),
        ("transfer 1000 rupies", "Transfer 1000 rupies."),
        // Mixed digit-word
        ("send 5 hundred rupees", "Send ₹500."),
        ("transfer 50 thousand rupees", "Transfer ₹50,000."),
        // Cents / paise
        ("fifty cents", "$0.50."),
        ("ten paise", "₹0.10."),
        // Currency with discount
        ("twenty percent off 100 dollars", "20% off $100."),
        // Negative currency
        ("minus 50 dollars", "-$50."),
        ("loss of 1 lakh rupees", "Loss of ₹1,00,000."),
        // Full sentence
        (
            "the new mac costs about 1 lakh 50 thousand rupees",
            "The new Mac costs about ₹1,50,000.",
        ),
    ];
    run_corpus("05-currency", cases, FormattingMode::Smart);
}

// ============================================================
// 6. TIME & DATE — 40 cases
// ============================================================

#[test]
fn corpus_06_time_date() {
    let cases: &[(&str, &str)] = &[
        // Time formatting
        ("at three pm", "At 3:00 PM."),
        ("at three thirty pm", "At 3:30 PM."),
        ("nine am", "9:00 AM."),
        ("twelve noon", "12:00 PM."),
        ("midnight", "12:00 AM."),
        // 24-hour
        ("at fifteen hundred", "At 15:00."),
        // With minutes
        ("five fifteen am", "5:15 AM."),
        ("ten forty five pm", "10:45 PM."),
        // Date words
        ("on Monday", "On Monday."),
        ("by Friday", "By Friday."),
        ("next Tuesday", "Next Tuesday."),
        ("last Wednesday", "Last Wednesday."),
        // Months
        ("in January", "In January."),
        ("on March third", "On March 3rd."),
        ("on March third 2024", "On March 3rd, 2024."),
        // Day-of-month ordinals
        ("the first of June", "The 1st of June."),
        ("the twenty fifth of December", "The 25th of December."),
        // Relative dates
        ("yesterday at 3 pm", "Yesterday at 3:00 PM."),
        ("tomorrow at noon", "Tomorrow at 12:00 PM."),
        ("in two weeks", "In 2 weeks."),
        ("in three months", "In 3 months."),
        // Hindi date words (preserved)
        ("kal mil te hai", "Kal mil te hai."),
        ("aaj subah meeting", "Aaj subah meeting."),
        // Durations
        ("for two hours", "For 2 hours."),
        ("for thirty minutes", "For 30 minutes."),
        ("for one and a half hours", "For 1.5 hours."),
        // Mixed
        (
            "meeting on Tuesday at three pm for one hour",
            "Meeting on Tuesday at 3:00 PM for 1 hour.",
        ),
        // Already formatted
        ("the meeting is at 3:00 PM", "The meeting is at 3:00 PM."),
        ("see you on March 5", "See you on March 5."),
        // No date — passthrough
        ("call me later", "Call me later."),
        // Year alone
        ("two thousand and twenty four", "2024."),
        ("nineteen ninety nine", "1999."),
        // Time with "o'clock"
        ("at three o'clock", "At 3:00."),
        ("eight o'clock sharp", "8:00 sharp."),
        // Quarter/half past
        ("quarter past three", "3:15."),
        ("half past four", "4:30."),
        ("quarter to six", "5:45."),
        // Date with year
        ("on March 5 2024", "On March 5, 2024."),
        // Decades
        ("in the eighties", "In the 80s."),
        ("the nineties were great", "The 90s were great."),
    ];
    run_corpus("06-time-date", cases, FormattingMode::Smart);
}

// ============================================================
// 7. LISTS — 50 cases
// ============================================================

#[test]
fn corpus_07_lists() {
    let cases: &[(&str, &str)] = &[
        // Grocery list trigger
        (
            "grocery list milk bread eggs",
            "- Milk\n- Bread\n- Eggs",
        ),
        (
            "grocery list milk, bread, eggs",
            "- Milk\n- Bread\n- Eggs",
        ),
        (
            "shopping list apples oranges and bananas",
            "- Apples\n- Oranges\n- Bananas",
        ),
        (
            "todo list call mom send email pay rent",
            "- Call mom\n- Send email\n- Pay rent",
        ),
        // Ordinal list
        (
            "first call mom second send email third pay rent",
            "- Call mom\n- Send email\n- Pay rent",
        ),
        (
            "first apples second oranges third bananas",
            "- Apples\n- Oranges\n- Bananas",
        ),
        (
            "first sign in second update profile third upload photo",
            "- Sign in\n- Update profile\n- Upload photo",
        ),
        // Comma-separated with "and"
        (
            "grocery list milk, bread and eggs",
            "- Milk\n- Bread\n- Eggs",
        ),
        // Just commas (not list intent — single sentence)
        ("apples, oranges, and bananas", "Apples, oranges, and bananas."),
        // Points are
        (
            "points are clarity, brevity, and impact",
            "- Clarity\n- Brevity\n- Impact",
        ),
        // Items are
        (
            "items are pen, notebook, charger",
            "- Pen\n- Notebook\n- Charger",
        ),
        // Mid-sentence "grocery list" should NOT trigger
        ("I left the grocery list at home", "I left the grocery list at home."),
        ("the shopping list is long", "The shopping list is long."),
        // Mid-sentence ordinal should NOT trigger
        ("we won the first prize and second place", "We won the first prize and second place."),
        // Single item list
        ("grocery list milk", "- Milk"),
        // List with quantities
        (
            "grocery list two apples three oranges and one banana",
            "- 2 apples\n- 3 oranges\n- 1 banana",
        ),
        // List inside email body
        (
            "Email to Raj saying first send invoice second update spreadsheet third confirm meeting",
            "Hi Raj,\n\n- Send invoice\n- Update spreadsheet\n- Confirm meeting\n\nBest regards,",
        ),
        // List with sub-items (not supported, just bullet flat)
        (
            "first thing main item second thing minor item",
            "- Thing main item\n- Thing minor item",
        ),
        // Trailing punctuation on items
        (
            "grocery list milk, bread, eggs.",
            "- Milk\n- Bread\n- Eggs",
        ),
        // List with ordinal 4-5
        (
            "first do A second do B third do C fourth do D fifth do E",
            "- Do A\n- Do B\n- Do C\n- Do D\n- Do E",
        ),
        // Empty list
        ("grocery list", ""),
        // List with currency items
        (
            "grocery list milk for 100 rupees, bread for 50 rupees",
            "- Milk for ₹100\n- Bread for ₹50",
        ),
        // Commas-only paragraph (no trigger)
        (
            "I bought milk, bread and eggs",
            "I bought milk, bread and eggs.",
        ),
        // Long list-style without trigger word
        (
            "milk bread eggs cheese butter",
            "Milk bread eggs cheese butter.",
        ),
        // List with multi-word items
        (
            "todo list buy a new laptop fix the bug call the client",
            "- Buy a new laptop\n- Fix the bug\n- Call the client",
        ),
        // Very long items
        (
            "first prepare the quarterly report second schedule the all-hands meeting",
            "- Prepare the quarterly report\n- Schedule the all-hands meeting",
        ),
        // Mixed-language items
        (
            "grocery list dudh, atta, chai",
            "- Dudh\n- Atta\n- Chai",
        ),
        // List with parenthetical
        (
            "grocery list milk (whole), bread (whole grain), eggs",
            "- Milk (whole)\n- Bread (whole grain)\n- Eggs",
        ),
        // List with numbers
        (
            "first 100 apples second 200 oranges third 300 bananas",
            "- 100 apples\n- 200 oranges\n- 300 bananas",
        ),
        // List with conjunction
        (
            "grocery list milk and bread and eggs",
            "- Milk\n- Bread\n- Eggs",
        ),
        // Trigger but no items
        ("grocery list nothing today", "- Nothing today"),
        // List intent with "are"
        (
            "the action items are review proposal sign contract send invoice",
            "- Review proposal\n- Sign contract\n- Send invoice",
        ),
        // Note intent (not list) — should NOT bullet
        (
            "note that the meeting is at 3 pm",
            "The meeting is at 3:00 PM.",
        ),
        // Lists with semicolon items (should still split on semicolons too)
        (
            "grocery list milk; bread; eggs",
            "- Milk\n- Bread\n- Eggs",
        ),
    ];
    run_corpus("07-lists", cases, FormattingMode::Smart);
}

// ============================================================
// 8. EMAILS — 40 cases
// ============================================================

#[test]
fn corpus_08_emails() {
    let cases: &[(&str, &str)] = &[
        // Basic
        (
            "email to Raj saying meeting at 5",
            "Hi Raj,\n\nMeeting at 5.\n\nBest regards,",
        ),
        (
            "email to Priya about the launch review",
            "Hi Priya,\n\nThe launch review.\n\nBest regards,",
        ),
        // Verbs
        (
            "write an email to Sarah saying thanks",
            "Hi Sarah,\n\nThanks.\n\nBest regards,",
        ),
        (
            "send a mail to John about the project",
            "Hi John,\n\nThe project.\n\nBest regards,",
        ),
        (
            "compose email to Mary saying we're delayed",
            "Hi Mary,\n\nWe're delayed.\n\nBest regards,",
        ),
        (
            "draft an email to Tom about the meeting",
            "Hi Tom,\n\nThe meeting.\n\nBest regards,",
        ),
        // No "to"
        (
            "email Raj about meeting tomorrow",
            "Hi Raj,\n\nMeeting tomorrow.\n\nBest regards,",
        ),
        // No recipient
        (
            "compose an email saying I'll be late",
            "Hi,\n\nI'll be late.\n\nBest regards,",
        ),
        (
            "write an email about the launch",
            "Hi,\n\nThe launch.\n\nBest regards,",
        ),
        // Body markers
        (
            "email to Priya: please review the proposal",
            "Hi Priya,\n\nPlease review the proposal.\n\nBest regards,",
        ),
        (
            "email to Raj, the meeting is moved",
            "Hi Raj,\n\nThe meeting is moved.\n\nBest regards,",
        ),
        // Preamble
        (
            "hey can you write an email to Raj saying we're delayed",
            "Hi Raj,\n\nWe're delayed.\n\nBest regards,",
        ),
        (
            "please draft an email to Mary about the proposal",
            "Hi Mary,\n\nThe proposal.\n\nBest regards,",
        ),
        // Multi-sentence body
        (
            "email to Raj saying meeting at 5. Please bring slides.",
            "Hi Raj,\n\nMeeting at 5. Please bring slides.\n\nBest regards,",
        ),
        // With currency in body
        (
            "email to vendor saying transfer 25 thousand rupees",
            "Hi Vendor,\n\nTransfer ₹25,000.\n\nBest regards,",
        ),
        // With list in body
        (
            "email to Raj saying first send invoice second update tracker",
            "Hi Raj,\n\n- Send invoice\n- Update tracker\n\nBest regards,",
        ),
        // With correction in body
        (
            "email to Raj saying send 100 dollars no wait send 200 dollars",
            "Hi Raj,\n\nSend $200.\n\nBest regards,",
        ),
        // Recipient with hyphen / apostrophe
        (
            "email to Mary-Anne about the proposal",
            "Hi Mary-Anne,\n\nThe proposal.\n\nBest regards,",
        ),
        (
            "email to O'Brien saying meeting at 5",
            "Hi O'Brien,\n\nMeeting at 5.\n\nBest regards,",
        ),
        // Lower-case recipient
        (
            "email to raj saying hello",
            "Hi Raj,\n\nHello.\n\nBest regards,",
        ),
        // Multi-word recipient
        (
            "email to John Smith about the meeting",
            "Hi John Smith,\n\nThe meeting.\n\nBest regards,",
        ),
        // Email mode without intent words — ?
        (
            "this is a test email body",
            "This is a test email body.",
        ),
        // Question in body
        (
            "email to Priya saying are you free Friday question mark",
            "Hi Priya,\n\nAre you free Friday?\n\nBest regards,",
        ),
        // No body marker — should not trigger email intent
        (
            "email Raj",
            "Email Raj.",
        ),
        (
            "email is broken",
            "Email is broken.",
        ),
        // Word "email" not at start
        (
            "I sent an email to Raj yesterday",
            "I sent an email to Raj yesterday.",
        ),
        // "Mail" instead of "email"
        (
            "mail to Sarah saying thanks",
            "Hi Sarah,\n\nThanks.\n\nBest regards,",
        ),
        // Forwarded-style
        (
            "forward this email to Raj",
            "Forward this email to Raj.",
        ),
        // Signature word
        (
            "email to Raj saying thanks. Best regards.",
            "Hi Raj,\n\nThanks. Best regards.\n\nBest regards,",
        ),
        // Negation in body
        (
            "email to Raj saying don't come tomorrow",
            "Hi Raj,\n\nDon't come tomorrow.\n\nBest regards,",
        ),
        // Long preamble
        (
            "I want you to write an email to Raj saying the meeting is cancelled",
            "Hi Raj,\n\nThe meeting is cancelled.\n\nBest regards,",
        ),
        // Email with em-dash body marker
        (
            "email to Raj — please review the doc",
            "Hi Raj,\n\nPlease review the doc.\n\nBest regards,",
        ),
        // Conversational
        (
            "ok so email to Raj saying see you Monday",
            "Hi Raj,\n\nSee you Monday.\n\nBest regards,",
        ),
        // Email with multi-paragraph body (using new paragraph)
        (
            "email to Raj saying see you Monday new paragraph also bring the report",
            "Hi Raj,\n\nSee you Monday\n\nAlso bring the report.\n\nBest regards,",
        ),
        // Recipient with title
        (
            "email to Dr Smith about the appointment",
            "Hi Dr Smith,\n\nThe appointment.\n\nBest regards,",
        ),
        // Spurious comma in body
        (
            "email to Raj saying, the meeting, is moved",
            "Hi Raj,\n\nThe meeting, is moved.\n\nBest regards,",
        ),
    ];
    run_corpus("08-emails", cases, FormattingMode::Smart);
}

// ============================================================
// 9. CODE & COMMANDS — 30 cases
// ============================================================

#[test]
fn corpus_09_code_commands() {
    let cases: &[(&str, &str)] = &[
        // Shell commands
        (
            "git commit dash m feat colon add login endpoint",
            "Git commit -m feat: add login endpoint.",
        ),
        ("npm install dash dash save", "Npm install --save."),
        ("ls dash la", "Ls -la."),
        // File paths (should not be reformatted)
        ("the file is at slash users slash siddhant", "The file is at /users/siddhant."),
        // URLs
        ("visit example dot com", "Visit example.com."),
        ("go to https colon slash slash example dot com", "Go to https://example.com."),
        // Code-like phrases
        ("set x equals 5", "Set x = 5."),
        ("if x greater than 10", "If x > 10."),
        ("call function open paren close paren", "Call function()."),
        // Variable names
        ("the variable is foo bar", "The variable is foo bar."),
        ("camelCase identifier", "CamelCase identifier."),
        // Symbols spoken
        ("hash tag spokn", "#spokn."),
        ("at the rate spokn", "@spokn."),
        ("ampersand", "&."),
        // Code in middle of sentence
        ("I ran git status this morning", "I ran git status this morning."),
        // Programming language
        ("the function returns null", "The function returns null."),
        // In Raw mode
        // (these would only work in Raw mode; since we test Smart, treat as basic Smart)
        // Markdown
        ("this is bold text", "This is bold text."),
        // SQL
        ("select star from users", "Select * from users."),
        // Array
        ("the array contains one two three", "The array contains 1 2 3."),
        // Boolean
        ("the value is true", "The value is true."),
        // JSON-style
        (
            "open brace key colon value close brace",
            "{key: value}.",
        ),
        // RegEx
        (
            "pattern dot star backslash s plus",
            "Pattern .*\\s+.",
        ),
        // Math
        ("two plus two equals four", "2 + 2 = 4."),
        ("five times six equals thirty", "5 × 6 = 30."),
        // Programming numbers
        ("zero indexed", "0-indexed."),
        // Special chars
        ("tilde slash dot ssh", "~/.ssh."),
        ("dot env file", ".env file."),
        // Spoken syntax
        (
            "open square bracket one two three close square bracket",
            "[1, 2, 3].",
        ),
        // Bash redirection
        ("echo hello redirect to file", "Echo hello > file."),
        // Quoted command
        (
            "the command is quote git status quote",
            "The command is \"git status\".",
        ),
        // Multi-line code (using new line)
        (
            "function add open paren a comma b close paren new line return a plus b",
            "Function add(a, b)\nReturn a + b.",
        ),
    ];
    run_corpus("09-code-commands", cases, FormattingMode::Smart);
}

// ============================================================
// 10. HINGLISH — 30 cases
// ============================================================

#[test]
fn corpus_10_hinglish() {
    let cases: &[(&str, &str)] = &[
        // Pure Hinglish
        ("haan bhai kal subah office chalo", "Haan bhai kal subah office chalo."),
        ("matlab abhi ja raha hu", "Matlab abhi ja raha hu."),
        ("theek hai chalo", "Theek hai chalo."),
        // Code-switching
        (
            "send the report kal subah",
            "Send the report kal subah.",
        ),
        (
            "I'll meet you at 5 pm theek hai",
            "I'll meet you at 5:00 PM theek hai.",
        ),
        (
            "yaar I need help with this",
            "Yaar I need help with this.",
        ),
        // Hindi numbers
        (
            "ek do teen char paanch",
            "1 2 3 4 5.",
        ),
        // Currency in Hinglish
        (
            "bhai send paanch sau rupees",
            "Bhai send ₹500.",
        ),
        (
            "transfer ek lakh rupees",
            "Transfer ₹1,00,000.",
        ),
        // Common Hinglish phrases
        ("kya karein aaj", "Kya karein aaj."),
        ("matlab kya bol raha hu", "Matlab kya bol raha hu."),
        // Filler removal in Hinglish
        ("haan bhai um kal milte hain", "Haan bhai kal milte hain."),
        // Self-correction in Hinglish
        (
            "kal milte hain no wait parso milte hain",
            "Parso milte hain.",
        ),
        // Email in Hinglish
        (
            "email to Raj saying kal subah meeting",
            "Hi Raj,\n\nKal subah meeting.\n\nBest regards,",
        ),
        // List in Hinglish
        (
            "grocery list dudh atta chai",
            "- Dudh\n- Atta\n- Chai",
        ),
        // Mixed currency + Hinglish
        (
            "bhai mai paanch hazaar rupees bhej deta hu",
            "Bhai mai ₹5,000 bhej deta hu.",
        ),
        // Hinglish question
        (
            "kya tum aa rahe ho question mark",
            "Kya tum aa rahe ho?",
        ),
        // Short Hinglish
        ("haan ji", "Haan ji."),
        ("nahi yaar", "Nahi yaar."),
        // Hinglish with English work terms
        (
            "meeting kal subah office mein hai",
            "Meeting kal subah office mein hai.",
        ),
        (
            "presentation ready kar do please",
            "Presentation ready kar do please.",
        ),
        // Numbers in Hinglish
        (
            "do hazaar rupees bhej do",
            "₹2,000 bhej do.",
        ),
        // Mixed time
        (
            "kal subah 9 baje aana",
            "Kal subah 9 baje aana.",
        ),
        // Pronouns
        (
            "main aaj nahi aa sakta",
            "Main aaj nahi aa sakta.",
        ),
        // Hindi-only sentence (pure Devanagari speakers using roman script)
        (
            "tum kaisa ho aaj",
            "Tum kaisa ho aaj.",
        ),
        // Hindi with English emotion
        (
            "yaar I'm so confused matlab kya kar raha hu",
            "Yaar I'm so confused matlab kya kar raha hu.",
        ),
        // Call to action
        (
            "chalo phir kal milte hain",
            "Chalo phir kal milte hain.",
        ),
        // Greetings
        (
            "namaste sab theek hai",
            "Namaste sab theek hai.",
        ),
        // Affirmation
        ("bilkul sahi kaha", "Bilkul sahi kaha."),
        // Long Hinglish
        (
            "haan bhai kal subah office chalo phir lunch karenge theek hai",
            "Haan bhai kal subah office chalo phir lunch karenge theek hai.",
        ),
    ];
    run_corpus("10-hinglish", cases, FormattingMode::Smart);
}

// ============================================================
// 11. APP-CONTEXT — 30 cases
// ============================================================

#[test]
fn corpus_11_app_context() {
    use crate::formatting::AppKind;
    let mut ctx = FormattingContext::default();
    let mut c = cfg(FormattingMode::Smart);
    c.detect_app_context = true;

    // Terminal forces Raw
    ctx.app_kind = AppKind::Terminal;
    let cases_terminal: &[(&str, &str)] = &[
        ("git commit dash m hello", "git commit dash m hello"),
        ("um, hello world", "um, hello world"),
        ("five hundred rupees", "five hundred rupees"),
        ("first thing second thing", "first thing second thing"),
    ];
    let mut failures = Vec::new();
    for (i, (input, expected)) in cases_terminal.iter().enumerate() {
        let actual = format(input, &c, &ctx);
        if actual.trim() != expected.trim() {
            failures.push(format!(
                "  [terminal {}] input: {:?}\n        expected: {:?}\n        actual: {:?}",
                i + 1,
                input,
                expected,
                actual
            ));
        }
    }

    // Code editor forces Raw
    ctx.app_kind = AppKind::Code;
    for (i, (input, expected)) in cases_terminal.iter().enumerate() {
        let actual = format(input, &c, &ctx);
        if actual.trim() != expected.trim() {
            failures.push(format!(
                "  [code {}] input: {:?}\n        expected: {:?}\n        actual: {:?}",
                i + 1,
                input,
                expected,
                actual
            ));
        }
    }

    // Search bar forces Raw
    ctx.app_kind = AppKind::Search;
    for (i, (input, expected)) in cases_terminal.iter().enumerate() {
        let actual = format(input, &c, &ctx);
        if actual.trim() != expected.trim() {
            failures.push(format!(
                "  [search {}] input: {:?}\n        expected: {:?}\n        actual: {:?}",
                i + 1,
                input,
                expected,
                actual
            ));
        }
    }

    // Email app upgrades to Email mode
    ctx.app_kind = AppKind::Email;
    let cases_email: &[(&str, &str)] = &[
        (
            "thank you for your time",
            "Hi,\n\nThank you for your time.\n\nBest regards,",
        ),
        (
            "the meeting is moved to friday",
            "Hi,\n\nThe meeting is moved to friday.\n\nBest regards,",
        ),
    ];
    for (i, (input, expected)) in cases_email.iter().enumerate() {
        let actual = format(input, &c, &ctx);
        if actual.trim() != expected.trim() {
            failures.push(format!(
                "  [email-app {}] input: {:?}\n        expected: {:?}\n        actual: {:?}",
                i + 1,
                input,
                expected,
                actual
            ));
        }
    }

    // Messaging app collapses Smart→Message
    ctx.app_kind = AppKind::Messaging;
    let cases_msg: &[(&str, &str)] = &[
        ("um hey can we meet at 5", "Hey can we meet at 5."),
        ("yaar kya kar rahe ho", "Yaar kya kar rahe ho."),
        ("send 100 dollars please", "Send $100 please."),
    ];
    for (i, (input, expected)) in cases_msg.iter().enumerate() {
        let actual = format(input, &c, &ctx);
        if actual.trim() != expected.trim() {
            failures.push(format!(
                "  [messaging {}] input: {:?}\n        expected: {:?}\n        actual: {:?}",
                i + 1,
                input,
                expected,
                actual
            ));
        }
    }

    // Notes / Slack
    ctx.app_kind = AppKind::Notes;
    let cases_notes: &[(&str, &str)] = &[
        ("um, hello world", "Hello world."),
        ("one two three", "1 2 3."),
    ];
    for (i, (input, expected)) in cases_notes.iter().enumerate() {
        let actual = format(input, &c, &ctx);
        if actual.trim() != expected.trim() {
            failures.push(format!(
                "  [notes {}] input: {:?}\n        expected: {:?}\n        actual: {:?}",
                i + 1,
                input,
                expected,
                actual
            ));
        }
    }

    // Unknown — uses user mode (Smart)
    ctx.app_kind = AppKind::Unknown;
    let cases_unknown: &[(&str, &str)] = &[
        ("um hello world", "Hello world."),
        ("five hundred rupees", "₹500."),
    ];
    for (i, (input, expected)) in cases_unknown.iter().enumerate() {
        let actual = format(input, &c, &ctx);
        if actual.trim() != expected.trim() {
            failures.push(format!(
                "  [unknown {}] input: {:?}\n        expected: {:?}\n        actual: {:?}",
                i + 1,
                input,
                expected,
                actual
            ));
        }
    }

    let total = cases_terminal.len() * 3 + cases_email.len() + cases_msg.len() + cases_notes.len() + cases_unknown.len();
    let passed = total - failures.len();
    if !failures.is_empty() {
        panic!(
            "\n=== 11-app-context ===\n{} of {} passed ({} failed)\n\n{}\n",
            passed,
            total,
            failures.len(),
            failures.join("\n\n")
        );
    }
}

// ============================================================
// 12. EDGE CASES — 30 cases
// ============================================================

#[test]
fn corpus_12_edge_cases() {
    let cases: &[(&str, &str)] = &[
        // Empty / whitespace
        ("", ""),
        (" ", " "),
        ("   ", "   "),
        // Single word
        ("hello", "Hello."),
        ("yes", "Yes."),
        // Single character
        ("a", "A."),
        // Just punctuation
        (".", "."),
        ("?", "?"),
        // Just numbers
        ("123", "123."),
        ("1 2 3", "1 2 3."),
        // Very long sentence
        (
            "this is a very long sentence with many many words that goes on and on and never seems to end no matter how much we keep adding to it which is the point of this test honestly",
            "This is a very long sentence with many words that goes on and on and never seems to end no matter how much we keep adding to it which is the point of this test honestly.",
        ),
        // Multiple sentences
        (
            "hello world. this is sentence two. and this is three.",
            "Hello world. This is sentence two. And this is three.",
        ),
        // Only punctuation words
        ("comma period question mark", "Comma. Question mark."),
        // Mix of newlines
        ("line one\nline two", "Line one\nLine two."),
        // Tabs (should collapse)
        ("hello\tworld", "Hello world."),
        // Multiple spaces
        ("hello     world", "Hello world."),
        // Unicode
        ("café résumé naïve", "Café résumé naïve."),
        // Emoji (should pass through)
        ("hello world 🎉", "Hello world 🎉."),
        // RTL (Hebrew/Arabic) — should pass through
        ("שלום עולם", "שלום עולם."),
        ("مرحبا بالعالم", "مرحبا بالعالم."),
        // Mixed scripts
        ("hello 世界", "Hello 世界."),
        // Repeated identical sentence
        (
            "this is a test this is a test this is a test",
            "This is a test this is a test this is a test.",
        ),
        // Numbers with commas
        ("1,000 dollars", "$1,000."),
        // Quotes around content
        (
            "she said \"hello world\"",
            "She said \"hello world\".",
        ),
        // Apostrophes
        ("don't won't can't", "Don't won't can't."),
        // Hyphenated compounds
        ("state-of-the-art design", "State-of-the-art design."),
        // Acronyms
        ("the USA and the UK", "The USA and the UK."),
        ("NASA launched a satellite", "NASA launched a satellite."),
        // Mixed case in middle
        ("the iPhone is great", "The iPhone is great."),
        // URL-like
        ("example.com is a website", "Example.com is a website."),
        // Email-like
        ("contact@spokn.app for help", "Contact@spokn.app for help."),
        // Already-formatted text
        (
            "Hello, world. How are you?",
            "Hello, world. How are you?",
        ),
        // Trailing whitespace
        ("hello world   ", "Hello world."),
        // Leading whitespace
        ("   hello world", "Hello world."),
    ];
    run_corpus("12-edge-cases", cases, FormattingMode::Smart);
}
