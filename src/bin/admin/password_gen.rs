use rand::Rng;
use rand::seq::SliceRandom;

pub fn generate_password() -> String {
    let mut rng = rand::thread_rng();

    //Create vector of all uppercase chars, lowercase chars, digits, special characters
    let upper = ('A'..='Z').collect::<Vec<_>>();
    let lower = ('a'..='z').collect::<Vec<_>>();
    let digit = ('0'..='9').collect::<Vec<_>>();
    let special = b"!@#$%^&*()-_=+[]{};:,.<>?"
        .iter()
        .map(|&b| b as char) //Map each as char
        .collect::<Vec<_>>();

    let upper_count = rng.gen_range(2..4);
    let lower_count = rng.gen_range(3..5);
    let digit_count = rng.gen_range(3..5);
    let special_count = rng.gen_range(2..4);

    //Next we just gonna choose randomly char from different collections n times.
    let mut password = Vec::new();
    //Sample without replacement, so no repeat
    password.extend(upper.choose_multiple(&mut rng, upper_count).cloned());
    password.extend(lower.choose_multiple(&mut rng, lower_count).cloned());
    password.extend(digit.choose_multiple(&mut rng, digit_count).cloned());
    password.extend(special.choose_multiple(&mut rng, special_count).cloned());

    password.shuffle(&mut rng); //Shuffle so you guys cannot guess any pridictable postition anymore. Hahaha

    password.into_iter().collect()
}
