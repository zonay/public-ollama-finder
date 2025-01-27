use anyhow::Result;
use console::style;
use crossterm::{cursor, terminal::{Clear, ClearType}, ExecutableCommand};
use std::io::Write;

// Border constants to match main.rs style
const BOX_HEADER: &str = "╭─ ";
const BOX_MIDDLE: &str = "├─ ";
const BOX_END: &str = "╰─ ";
const BULLET: &str = "• ";
const SUB_ITEM: &str = "   ";

pub fn display_disclaimer() -> Result<bool> {
    let mut stdout = std::io::stdout();
    stdout.execute(Clear(ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;

    // Header section with better spacing
    println!("\n{}{}\n", BOX_HEADER, style("IMPORTANT NOTICE").red().bold());
    
    // Section 1 - Purpose
    println!("{}Before proceeding:", BOX_MIDDLE);
    println!("{}This tool is for educational and authorized", SUB_ITEM);
    println!("{}security testing purposes only.", SUB_ITEM);
    println!();
    
    // Section 2 - Warnings with improved spacing
    println!("{}Critical Warning:", BOX_MIDDLE);
    println!("{}{}", SUB_ITEM, style("Scanning servers without explicit permission").red().bold());
    println!("{}may result in serious consequences:", SUB_ITEM);  // Fixed: removed to_string() and fixed format
    println!("{}{} {}", SUB_ITEM, BULLET, style("Legal actions and prosecution").red());
    println!("{}{} {}", SUB_ITEM, BULLET, style("Network-wide IP bans").red());
    println!("{}{} {}", SUB_ITEM, BULLET, style("Defensive countermeasures").red());
    println!();
    
    // Section 3 - Server Information
    println!("{}Ollama instances are personal servers:", BOX_MIDDLE);
    println!("{}{} All access attempts are logged", SUB_ITEM, BULLET);
    println!("{}{} Resources are monitored", SUB_ITEM, BULLET);
    println!("{}{} Rate limits are enforced", SUB_ITEM, BULLET);
    println!();
    
    // Section 4 - Usage Guidelines
    println!("{}Responsible Usage Requirements:", BOX_MIDDLE);
    println!("{}{} Only scan authorized networks", SUB_ITEM, BULLET);
    println!("{}{} Follow best practices", SUB_ITEM, BULLET);
    println!("{}{} Respect system administrators", SUB_ITEM, BULLET);
    println!();

    // Agreement section with clear separation
    println!("{}{}", BOX_MIDDLE, style("LEGAL CONFIRMATION:").red().bold());
    println!("{}By proceeding, you explicitly confirm:", SUB_ITEM);
    println!("{}1. {}", SUB_ITEM, style("I have authorization for all target networks").red());
    println!("{}2. {}", SUB_ITEM, style("I accept full responsibility for my actions").red());
    println!("{}3. {}", SUB_ITEM, style("I understand all legal implications").red());
    println!();

    // Final prompt
    print!("{}{} ", BOX_END, style("Type 'y' to accept these terms:").bold());
    stdout.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("\n{}", style("Access denied: Agreement required to proceed.").red().bold());
        return Ok(false);
    }
    
    Ok(true)
}
