# Herbarium Specimen Duplicate Finder and Capture Tool

This application is designed to help speed up herbarium specimen digitization by reusing existing data for specimen duplicates. If a duplicate of your specimen has already been databased in another collection, why capture all that data again? Simply look up the record and reuse.

## How the Tool Works

See the full documentation at ...

The application uses data downloaded from GBIF as a reference database for finding duplicates. The interface is split into two parts:

1. Duplicates search (left side): Use this panel to search the reference database. When you enter search filters, the application queries the reference records to identify potential duplicates.
2. Selecting and Editing (right side): Clicking on a record from the search results on the left loads that specimen's details into the capture form on the right. You can review the details, edit or clean up fields, and save the record to your current capturing session.

Once you have finished capturing specimens for a session, you can export your captured records to a standard CSV file. This CSV file can then be imported into your main herbarium database system.

## Reference data [coming soon]

The data in the reference database need to be prepared from a GBIF download. Go to GBIF, select the collections and geographic area you would like to work with, and download the data. Then you need to prepare the data using the scripts in .... This includes standardizing collector names and cleaning up collection codes. Then import the dataset into the tool and you're ready to get started.

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
   Ensure you have the Rust compiler installed on your system. You can download it from https://www.rust-lang.org/. Then go to [tauri.app](https://tauri.app/start/prerequisites/) to make sure you have all the necessary prerequisites installed.

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

That said, it's often easier to make simple changes directly to the files in your IDE, if you know how to do so and if you can do it quickly. General rule of thumb: Big functionality changes and fixes that touch multiple files - use AI. Small UI updates - just update the code manually.

## Deployment

The project is set up with the updater plugin to allow for automatic updates. This requires the relevant public private key pair to sign app on build. See [the Tauri building instructions](https://v2.tauri.app/plugin/updater/#building) for details.

We're using GitHub Actions for deployment. When ready to deploy a new version (and make absolutely sure you're ready!), do the following:

```
git add .
git commit -m "Release v0.2.0"

git tag app-v0.2.0

git push origin main
git push origin app-v0.2.0
```

[!CAUTION] Remember to update the version number in the tauri config file!!!

## Deployment database

As the database is too large to bundle with the app installer via Github, we host it separately and the user needs to download and unzip the database file at a suitable location on their computer before using the app.
