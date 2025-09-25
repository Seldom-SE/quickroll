use chumsky::{extra::Err, prelude::*};
use rand::random_range;
use serenity::{
    all::{
        Command, CommandOptionType, CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage, Interaction, Message, Ready, ResolvedOption,
        ResolvedValue,
    },
    async_trait,
    prelude::*,
};
use text::int;
use tokio::main;

#[derive(Debug)]
enum Term {
    Dice { count: u32, size: u32 },
    Literal(i32),
}

#[derive(Debug)]
struct Roll {
    terms: Vec<Term>,
    advantageousness: i32,
}

fn message_parser<'a>() -> impl Parser<'a, &'a str, String, Err<Rich<'a, char>>> {
    one_of("rR")
        .ignore_then(any().repeated().collect::<String>())
        .then_ignore(end())
}

fn expr_parser<'a>() -> impl Parser<'a, &'a str, Roll, Err<Rich<'a, char>>> {
    let num = int(10).map(|int: &str| int.parse().unwrap());
    let signed = just('-').or_not().then(num).map(|(neg, num)| {
        if neg.is_some() {
            -(num as i32)
        } else {
            num as i32
        }
    });

    let ending = just('a')
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|r#as| r#as.len() as i32)
        .or(just('d')
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|ds| -(ds.len() as i32)))
        .or_not()
        .map(|advantageousness| advantageousness.unwrap_or_default())
        .then_ignore(just('/').then(any().repeated()).or_not())
        .then_ignore(end());

    signed
        .map(|num| vec![Term::Dice { count: 1, size: 20 }, Term::Literal(num)])
        .then(ending)
        .or(num
            .or_not()
            .then(just('d').ignore_then(num))
            .map(|(count, size)| {
                if let Some(count) = count {
                    Term::Dice { count, size }
                } else {
                    Term::Dice { count: 1, size }
                }
            })
            .or(signed.map(Term::Literal))
            .separated_by(just('+'))
            .at_least(1)
            .collect::<Vec<Term>>()
            .then(ending))
        .or(ending
            .map(|advantageousness| (vec![Term::Dice { count: 1, size: 20 }], advantageousness)))
        .map(|(terms, advantageousness)| Roll {
            terms,
            advantageousness,
        })
}

struct Maximality {
    max: bool,
    min: bool,
}

fn roll(expr: &str) -> Result<String, String> {
    match expr_parser().parse(expr).into_result() {
        Ok(roll) => {
            if roll.terms.iter().any(|term| {
                if let &Term::Dice { size, .. } = term {
                    size == 0
                } else {
                    false
                }
            }) {
                return Err("cannot roll dice of size 0".to_string());
            }

            let rolls = (0..=roll.advantageousness.abs()).map(|_| {
            roll.terms
                .iter()
                .map(|term| match *term {
                    Term::Dice { count, size } => {
                        (0..count).map(|_| random_range(1..=size)).fold(
                            (
                                0,
                                String::new(),
                                Maximality {
                                    max: true,
                                    min: true,
                                },
                                0,
                            ),
                            |(sum, string, mut maximality, count), roll| {
                                let mut crit = false;

                                if roll > 1 {
                                    maximality.min = false;
                                    crit ^= true;
                                }

                                if roll < size {
                                    maximality.max = false;
                                    crit ^= true;
                                }

                                (
                                    sum + roll as i32,
                                    format!(
                                        "{string}{}{}",
                                        if string.is_empty() { "" } else { " " },
                                        if crit {
                                            format!("**{roll}**")
                                        } else {
                                            roll.to_string()
                                        }
                                    ),
                                    maximality,
                                    count + 1,
                                )
                            },
                        )
                    }
                    Term::Literal(literal) => (
                        literal,
                        literal.to_string(),
                        Maximality {
                            max: true,
                            min: true,
                        },
                        1,
                    ),
                })
                .fold(
                    (
                        0,
                        String::new(),
                        Maximality {
                            max: true,
                            min: true,
                        },
                        0,
                    ),
                    |(sum, string, maximality, count), (term_sum, term, term_maximality, term_count)| {
                        (
                            sum + term_sum,
                            format!(
                                "{string}{}{term}",
                                if string.is_empty() { "" } else { " + " },
                            ),
                            Maximality {
                                max: maximality.max && term_maximality.max,
                                min: maximality.min && term_maximality.min,
                            },
                            count + term_count,
                        )
                    },
                )
            }).collect::<Vec<_>>();

            let (taken, _) = rolls
                .iter()
                .enumerate()
                .max_by_key(|(_, (sum, _, _, _))| {
                    if roll.advantageousness >= 0 {
                        *sum
                    } else {
                        -*sum
                    }
                })
                .unwrap();

            Ok(rolls.into_iter().enumerate().fold(
                #[cfg(feature = "debug")]
                format!("`{roll:?}`"),
                #[cfg(not(feature = "debug"))]
                String::new(),
                |string, (index, (sum, roll, maximality, count))| {
                    let delim = if index == taken { "" } else { "~~" };

                    format!(
                        "{string}{}{delim}{roll}{}{delim}",
                        if string.is_empty() { "" } else { " " },
                        if count > 1 {
                            format!(
                                " = {}",
                                if maximality.max != maximality.min {
                                    format!("**{sum}**")
                                } else {
                                    sum.to_string()
                                }
                            )
                        } else {
                            String::new()
                        }
                    )
                },
            ))
        }
        Err(errs) => {
            #[cfg(feature = "debug")]
            for err in &errs {
                eprintln!("{err}");
            }

            Err(errs
                .into_iter()
                .map(|err| err.to_string())
                .reduce(|err_1, err_2| format!("{err_1}; {err_2}"))
                .unwrap_or_else(|| "error".to_string()))
        }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        if let Err(err) =
            Command::create_global_command(
                &ctx.http,
                CreateCommand::new("r").description("roll dice").add_option(
                    CreateCommandOption::new(CommandOptionType::String, "dice", "dice expression"),
                ),
            )
            .await
        {
            eprintln!("failed to create command: {err}")
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let Some(expr) = message_parser().parse(&msg.content).into_output() else {
            return;
        };

        let Ok(res) = roll(&expr) else {
            return;
        };

        if let Err(err) = msg.reply(&ctx.http, res).await {
            eprintln!("error sending message: {err:?}");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction
            && command.data.name == "r"
        {
            let options = command.data.options();

            if let Err(err) = match roll(
                if let Some(ResolvedOption {
                    value: ResolvedValue::String(expr),
                    ..
                }) = options.first()
                {
                    expr
                } else {
                    ""
                },
            ) {
                Ok(res) => {
                    command
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content(res),
                            ),
                        )
                        .await
                }
                Err(err) => {
                    command
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new()
                                    .content(err)
                                    .ephemeral(true),
                            ),
                        )
                        .await
                }
            } {
                eprintln!("failed to respond to slash command: {err}");
            }
        }
    }
}

#[main]
async fn main() {
    let token = include_str!("../TOKEN.txt");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .await
        .expect("err creating client");

    if let Err(err) = client.start().await {
        eprintln!("client error: {err:?}");
    }
}
