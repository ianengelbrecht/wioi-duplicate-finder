# Herbarium Specimen Duplicate Finder and Capture Tool

This application is designed for botanical researchers and herbarium curators to capture specimen data and automatically identify duplicate specimens. It helps standardise specimen data and streamlines the digitisation workflow.

## How the Tool Works

The application interface is split into two primary columns:

1. Finding Duplicates (Left Column): Use this panel to search the reference databases (which contain specimen records from GBIF and taxonomic names from the Kew World Checklist of Vascular Plants). When you enter search filters, the application queries the reference records to identify potential duplicates.
2. Selecting and Editing (Right Column): Clicking on a record from the search results on the left loads that specimen's details into the capture form on the right. You can review the details, edit or clean up fields (such as dates, taxonomies, and geographic hierarchies), and save the record to your current capturing session.

Once you have finished capturing specimens for a session, you can export your captured records to a standard CSV file. This CSV file can then be imported into your main herbarium database system.

## How to Install and Run the Tool

### Running from the Installer
An installer package will be linked here once the production file is uploaded. Installing this version handles all setup steps automatically.

### Running from Source Code
If you want to run the application from source code instead of using the installer, follow these steps:

1. Clone the repository:
   git clone <repository-url>

2. Install Node.js dependencies:
   From the project root directory, run:
   npm install

3. Install Rust:
   Ensure you have the Rust compiler installed on your system. You can download it from https://www.rust-lang.org/

4. Start the application:
   Run the development command from the project root directory:
   npm run tauri dev

## Development and Customisation

This project is designed to be maintained and developed using the Antigravity AI coding assistant.

If you need to make changes, add features, or update validation rules, you can invoke Antigravity and describe the changes you want in plain English. For example, you can tell the AI to:
- Add a new input field to the capture form.
- Modify the database schema.
- Update search validation logic or autocomplete behaviors.

The AI will handle updating the Svelte frontend, the Rust backend, and database migrations automatically.
