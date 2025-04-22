pub mod ping {
    use serenity::all::{CreateCommand, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("ping")
            .description("ping wawa")
            .kind(serenity::all::CommandType::ChatInput)
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        "pong!".to_string()
    }
}

pub mod version {
    use serenity::all::{CreateCommand, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("version")
            .description("wawa's version")
            .kind(serenity::all::CommandType::ChatInput)
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        uiua::VERSION.to_string()
    }
}

pub mod help {
    use serenity::all::{CreateCommand, ResolvedOption};

    use crate::HELP_MESSAGE;

    pub fn register() -> CreateCommand {
        CreateCommand::new("help")
            .description("display wawa's help text")
            .kind(serenity::all::CommandType::ChatInput)
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        HELP_MESSAGE.to_string()
    }
}

pub mod format {
    use serenity::all::{CreateCommand, CreateCommandOption, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("format")
            .description("format uiua code")
            .kind(serenity::all::CommandType::ChatInput)
            .add_option(CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                "code",
                "the code to be formatted",
            ))
    }
    pub fn run(options: &[ResolvedOption]) -> String {
        "unimplemented".to_string()
    }
}

pub mod pad {
    use serenity::all::{CreateCommand, CreateCommandOption, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("pad")
            .description("generate pad link of code")
            .kind(serenity::all::CommandType::ChatInput)
            .add_option(CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                "code",
                "the code to be pad-ed",
            ))
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        "unimplemented".to_string()
    }
}

pub mod docs {
    use serenity::all::{CreateCommand, CreateCommandOption, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("docs")
            .description("find docs of function")
            .kind(serenity::all::CommandType::ChatInput)
            .add_option(CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                "docs",
                "the function whose docs to get",
            ))
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        "unimplemented".to_string()
    }
}

pub mod emojify {
    use serenity::all::{CreateCommand, CreateCommandOption, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("emojify")
            .description("emojify code")
            .kind(serenity::all::CommandType::ChatInput)
            .add_option(CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                "code",
                "the code to be emojifiy-ed",
            ))
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        "unimplemented".to_string()
    }
}

pub mod run {
    use serenity::all::{CreateCommand, CreateCommandOption, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("run")
            .description("run code")
            .kind(serenity::all::CommandType::ChatInput)
            .add_option(CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                "code",
                "the code to be ran",
            ))
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        "unimplemented".to_string()
    }
}

pub mod show {
    use serenity::all::{CreateCommand, CreateCommandOption, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("show")
            .description("run code without displaying the source")
            .kind(serenity::all::CommandType::ChatInput)
            .add_option(CreateCommandOption::new(
                serenity::all::CommandOptionType::String,
                "code",
                "the code to be ran",
            ))
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        "unimplemented".to_string()
    }
}

/// This doesn't actually shutdown
pub mod shutdown {
    use serenity::all::{CreateCommand, ResolvedOption};

    pub fn register() -> CreateCommand {
        CreateCommand::new("shutdown")
            .description("shutdown wawa")
            .kind(serenity::all::CommandType::ChatInput)
    }
    pub fn run(_options: &[ResolvedOption]) -> String {
        "Ok, shutting down now...".to_string()
    }
}
