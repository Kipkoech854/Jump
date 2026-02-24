Jump 
Jump is a lightning-fast, fuzzy-matching directory navigator written in Rust. It learns your most frequently visited paths and lets you "jump" to them instantly, saving you thousands of keystrokes.Why Jump?
Blazing Fast: Built with Rust for instant startup and low memory footprint.
Smart Learning: Uses a local SQLite database to track frequency and recency.
Fuzzy Matching: Type j work to find ~/Documents/Dev/Work. Self-Healing: Automatically cleans up stale database entries via a background cron job.
InstallationOption 

1: Download Pre-built (Linux x86_64)Go to the Releases page.Download the latest jump_vX.X.X_linux_x86_64.zip.Unzip and run the installer:

  Bashunzip jump_v1.0.zip
  cd jump_v1.0
  ./install.sh
  source ~/.bashrc
  
Option 2: Build from SourceIf you are on a different architecture (ARM, macOS, etc.) or want to customize the code:

 Clone the Repository:
  git clone https://github.com/Kipkoech854/Jump.git
  cd jump
  
Compile the Binary
  cargo build --release
  
Run the InstallerThe project includes an automated script that handles the setup for you. Just copy the compiled binary to the root and run the script:
Bash#
 # 1. Make the installer executable
 chmod +x install.sh

 # 2. Run the automated installer
 ./install.sh

 # 3. Reload your shell
 source ~/.bashrc
Note: The install.sh script automatically sets up the j alias, configures your $PATH, and initializes the database.📖 UsageBasic NavigationJump to a directory by typing a partial name:Bash
j down      # Jumps to ~/Downloads
j proj      # Jumps to ~/Projects
j rus       # Jumps to ~/Projects/rust_stuff
Advanced CommandsCommandDescriptionj
  <path>Fuzzy search and jump to best match.
  j --retReturn to the previous directory (like cd - but smarter).
  j --cleanManually run database cleanup (removes non-existent paths).j <path> -e "<cmd>"Jump to path and execute a command (e.g., j work -e "ls -la").
  🔧 ConfigurationJump uses sensible defaults, but you can override them with environment variables if needed.
  DATABASE_URL: Path to the SQLite database.Default: 
  $HOME/jump.db 
  
  ContributingContributions are welcome!Fork the repository.
  Create a feature branch (git checkout -b feature/amazing-feature).
  Commit your changes (git commit -m 'Add amazing feature').Push to the branch (git push origin feature/amazing-feature).
  Open a Pull Request.

  
  LicenseDistributed under the MIT License.  See LICENSE for more information.
