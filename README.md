# Anime Downloader

A command-line tool written in Rust for downloading anime episodes from various anime streaming sites.

## Features

- Search for anime series by name
- Downloads entire series
- Supports multiple streaming sites (currently supports AnimePahe)
- Uses aria2c for efficient downloading
- Handles DDOS protection and cookies

## Prerequisites

### For Users
- aria2c installed and available in PATH

### For Development
- Rust toolchain (cargo, rustc)
- aria2c installed and available in PATH

## Usage

### Using Pre-built Binary
1. Download the latest release from the releases page
2. Extract the binary to a location of your choice
3. Run the program:
```bash
./anime
```

### Building from Source
1. Run the program:
```bash
cargo run
```

Once running:
1. Enter the name of the anime you want to download when prompted
2. Select the anime from the search results by entering its number
3. The program will create a directory with the anime name and download all episodes automatically

## Project Structure

- `src/`
  - `main.rs` - Entry point and CLI interface
  - `cookies.json` - Starter Cookies for the site.
  - `data.json` - A data file for ddos.
  - `common/` - Common utilities and traits
    - `anime.rs` - Core anime traits and interfaces
    - `download.rs` - Download management
    - `errors.rs` - Error handling
    - `quality.rs` - Video quality management
  - `ddos/` - DDOS protection handling
    - `ddos_guard_net.rs` - DDOS-Guard.net specific implementation
  - `download/` - Download implementations
    - `aria2c.rs` - aria2c downloader implementation
  - `sites/` - Anime site implementations
    - `animepahe.rs` - AnimePahe site implementation

## Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to your branch
5. Create a Pull Request

## License

This project is licensed under MIT

## Acknowledgments

- Built using Rust and Tokio
- Uses aria2c for downloading
- Inspired by various anime downloading tools and APIs
