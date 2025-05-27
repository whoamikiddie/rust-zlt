# ZLT GitHub Workflows

## Windows MSI Installer Build

This repository contains a GitHub Actions workflow that automatically builds a Windows MSI installer for the ZLT System Monitor application.

### How It Works

The workflow (`build-msi.yml`) does the following:

1. Runs on a Windows virtual machine in GitHub's cloud
2. Installs Rust and the necessary toolchain
3. Installs WiX Toolset for creating MSI installers
4. Builds the ZLT application in release mode
5. Creates an MSI installer using the WiX configuration
6. Uploads the installer as a GitHub artifact
7. If triggered by a tag, creates a GitHub Release with the installer

### How to Use

#### Automatic Builds

The workflow runs automatically when:
- You push to the main/master branch
- You create a pull request to main/master
- You create a new tag (which will also create a GitHub Release)

#### Manual Builds

You can also trigger the workflow manually:
1. Go to your GitHub repository
2. Click on "Actions"
3. Select "Build Windows MSI Installer" from the list
4. Click "Run workflow" 
5. Select the branch you want to build from
6. Click "Run workflow"

### Accessing the Installer

After the workflow completes:
1. Go to the workflow run in the Actions tab
2. Scroll down to "Artifacts"
3. Download "zlt-windows-installer"
4. Extract the ZIP file to get your MSI installer

### Creating Releases

To create a tagged release with the installer:
1. Create and push a new tag:
   ```
   git tag v1.0.0
   git push origin v1.0.0
   ```
2. The workflow will run and automatically create a GitHub Release with the installer attached

### Customizing the Installer

The installer is built using the WiX configuration files in the `wix/` directory. You can customize these files to change how the installer works.
