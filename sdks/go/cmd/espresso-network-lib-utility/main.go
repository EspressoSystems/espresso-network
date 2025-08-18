package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"
	"syscall"

	"github.com/spf13/cobra"
	"golang.org/x/mod/module"
)

const targetLib = "./verification/target/lib"
const baseURL = "https://github.com/EspressoSystems/espresso-network/releases"
const espressoModule = "github.com/EspressoSystems/espresso-network/sdks/go"

func main() {
	var version string
	var url string
	var destination string

	var rootCmd = &cobra.Command{Use: "app"}
	var downloadCmd = &cobra.Command{
		Use:   "download",
		Short: "Download the static library",
		Run: func(cmd *cobra.Command, args []string) {
			download(version, url, destination)
		},
	}
	downloadCmd.Flags().StringVarP(&version, "version", "v", "latest", "Specify the version to download")
	downloadCmd.Flags().StringVarP(&url, "url", "u", "", "Specify the url to download. If this is set, the version flag will be ignored")
	downloadCmd.Flags().StringVarP(&destination, "destination", "d", "./", "Specify the destination to download the library to")

	var cleanCmd = &cobra.Command{
		Use:   "clean",
		Short: "Clean the downloaded files",
		Run: func(cmd *cobra.Command, args []string) {
			clean(version)
		},
	}
	cleanCmd.Flags().StringVarP(&version, "version", "v", "latest", "Specify the version to clean. If this is not set, it will clean the latest version")

	var filePath string
	var checkSum string
	var linkCmd = &cobra.Command{
		Use:   "link",
		Short: "Create a symlink to the downloaded library",
		Run: func(cmd *cobra.Command, args []string) {
			createSymlink(filePath, checkSum, version)
		},
	}
	linkCmd.Flags().StringVarP(&filePath, "filePath", "f", "", "Specify the file path to create the symlink in")
	linkCmd.Flags().StringVarP(&checkSum, "checkSum", "c", "", "Specify the checkSum to create the symlink in")
	linkCmd.Flags().StringVarP(&version, "version", "v", "latest", "Specify the version to create the symlink for")

	rootCmd.AddCommand(downloadCmd, cleanCmd, linkCmd)
	err := rootCmd.Execute()
	if err != nil {
		fmt.Printf("Failed to execute command: %s\n", err)
		os.Exit(1)
	}
}

// ensureGoCacheDirectoryExists ensures that the parent directory exists, and
// has the correct permissions for modification to the underlying go cache
// directory.
func ensureGoCacheDirectoryExists(path string) (cleanup func(), err error) {
	dir, err := os.Stat(path)
	if err == nil {
		if dir.IsDir() {
			// The directory already exists, let's make sure we have the right
			// permissions to modify it.

			originalPerm := dir.Mode().Perm()
			if originalPerm&0700 != 0700 {
				// We don't have the ability to modify this directory, let's
				// change the permissions to allow us to do so.
				if err := os.Chmod(path, originalPerm|0700); err != nil {
					return func() {}, &os.PathError{Op: "chmod", Path: path, Err: err}
				}

				// Permissions changed successfully, we can now
				// modify the directory.
				// Return a cleanup function to restore the original
				// permissions.
				return func() {
					// Change the permissions back to the original permissions
					// after we are done
					if err := os.Chmod(path, originalPerm); err != nil {
						fmt.Printf("Failed to change permissions of %s: %s\n", path, err)
					}
				}, nil
			}

			return func() {}, nil
		}

		return func() {}, &os.PathError{Op: "mkdir", Path: path, Err: syscall.ENOTDIR}
	}

	// We need to create this directory.
	// First, let's ensure that our parent directory exists.
	parentPath := filepath.Dir(path)
	parentCleanup, err := ensureGoCacheDirectoryExists(parentPath)
	if err != nil {
		return parentCleanup, err
	}

	if err := os.Mkdir(path, 0755); err != nil {
		return parentCleanup, err
	}

	return func() {
		// Change the permissions back to the default go cache permissions
		// after we are done
		if err := os.Chmod(path, 0555); err != nil {
			fmt.Printf("Failed to change permissions of %s: %s\n", path, err)
		}

		parentCleanup()
	}, nil
}

// resolveVersion resolves the version to use for the symlink or download.
// If the version is "latest" or empty, it fetches the latest Go SDK release
// tag.
// If a specific version is provided, it returns that version.
func resolveVersion(version string) string {
	if version != "latest" && version != "" {
		return version
	}
	latestTag, err := FetchLatestGoSDKTag()
	if err != nil {
		fmt.Printf("Failed to fetch latest Espresso Go SDK release tag: %s\n", err)
		os.Exit(1)
	}
	return latestTag
}

func createSymlink(path, checkSum, version string) {
	version = resolveVersion(version)
	linkName := getFileName()
	fileDir := getFileDir(version)
	linkPath := filepath.Join(fileDir, linkName)

	if !filepath.IsAbs(path) {
		absPath, err := filepath.Abs(path)
		if err != nil {
			fmt.Printf("Failed to get absolute path: %s\n", err)
			os.Exit(1)
		}
		path = absPath
	}

	if _, err := os.Stat(linkPath); err == nil {
		fmt.Printf("Symlink %s already exists\n, Run clean to remove it first.\n", linkPath)
		return
	}

	// Check if the target file exists and is a regular file
	fileInfo, err := os.Stat(path)
	if err != nil {
		fmt.Printf("Target file does not exist: %s\n", path)
		os.Exit(1)
	}
	if !fileInfo.Mode().IsRegular() {
		fmt.Printf("Target file is not a regular file: %s\n", path)
		os.Exit(1)
	}

	// Check if the target file matches the checksum
	file, err := os.Open(path)
	if err != nil {
		fmt.Printf("Failed to open target file: %s\n", err)
		os.Exit(1)
	}
	defer file.Close()

	checksum, err := hashFile(file)
	if err != nil {
		fmt.Printf("Failed to calculate checksum: %s\n", err)
		os.Exit(1)
	}
	if checksum != checkSum {
		fmt.Printf("Checksum mismatch: %s != %s\n", checksum, checkSum)
		os.Exit(1)
	}

	permissionCleanup, err := ensureGoCacheDirectoryExists(fileDir)
	defer permissionCleanup()
	if err != nil {
		fmt.Printf("Failed to create target directory: %s\n", err)
		os.Exit(1)
	}

	err = os.Symlink(path, linkPath)
	if err != nil {
		fmt.Printf("Failed to create symlink: %s\n", err)
		os.Exit(1)
	}

	fmt.Printf("Created symlink: %s\n", linkPath)
}

func hashFile(file *os.File) (string, error) {
	// Ensure we read from the beginning of the file
	if _, err := file.Seek(0, io.SeekStart); err != nil {
		return "", err
	}
	hasher := sha256.New()
	if _, err := io.Copy(hasher, file); err != nil {
		return "", err
	}
	sum := hasher.Sum(nil)
	return hex.EncodeToString(sum), nil
}

func download(version string, specifiedUrl string, destination string) {
	fileName := getFileName()

	var url string
	if specifiedUrl != "" {
		fmt.Printf("Using specified url to download the library: %s\n", specifiedUrl)
		url = specifiedUrl
	} else {
		if version == "latest" {
			fmt.Printf("Using latest version %s to download the library\n", version)
		}
		version = resolveVersion(version)
		if strings.HasPrefix(version, "v") {
			version = fmt.Sprintf("sdks/go/%s", version)
		}
		url = fmt.Sprintf("%s/download/%s/%s", baseURL, version, fileName)
	}

	fmt.Printf("Downloading library from %s\n", url)
	resp, err := http.Get(url)
	if err != nil {
		fmt.Printf("Failed to download static library: %s\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	out, err := os.Create(filepath.Join(destination, fileName))
	if err != nil {
		fmt.Printf("Failed to create file: %s\n", err)
		os.Exit(1)
	}
	defer out.Close()

	_, err = io.Copy(out, resp.Body)
	if err != nil {
		fmt.Printf("Failed to write file: %s\n", err)
		os.Exit(1)
	}

	fmt.Printf("Verification library downloaded to: %s\n", destination)
}

func clean(version string) {
	version = resolveVersion(version)
	fileDir := getFileDir(version)
	permissionCleanup, err := ensureGoCacheDirectoryExists(fileDir)
	defer permissionCleanup()
	if err != nil {
		fmt.Printf("Failed to create target directory: %s\n", err)
		os.Exit(1)
	}

	files, err := os.ReadDir(fileDir)
	if err != nil {
		fmt.Printf("Failed to read directory: %s\n", err)
		os.Exit(1)
	}

	for _, file := range files {
		if err := os.Remove(filepath.Join(fileDir, file.Name())); err != nil {
			fmt.Printf("Failed to remove file %s: %s\n", file.Name(), err)
			os.Exit(1)
		}

		fmt.Printf("Removed file: %s\n", file.Name())
	}

	fmt.Println("Cleaned the symlink.")
}

func getArchitecture() string {
	switch runtime.GOARCH {
	default:
		panic(fmt.Sprintf("unsupported architecture: %s", runtime.GOARCH))
	case "amd64":
		return "x86_64"
	case "arm64":
		return "aarch64"
	}
}

func getLibraryExtension() string {
	switch runtime.GOOS {
	case "darwin":
		return ".dylib"
	case "linux":
		return ".so"
	default:
		panic(fmt.Sprintf("unsupported OS: %s", runtime.GOOS))
	}
}

func getCompilationTarget() string {
	switch runtime.GOOS {
	default:
		panic(fmt.Sprintf("unsupported OS: %s", runtime.GOOS))
	case "darwin":
		return "apple-darwin"
	case "linux":
		return "unknown-linux-musl"
	}
}

func getFileName() string {
	return fmt.Sprintf("libespresso_crypto_helper-%s-%s%s", getArchitecture(), getCompilationTarget(), getLibraryExtension())
}

// getGoModRootPath returns the root path of the Go module cache.
// It checks the GOMODCACHE environment variable first, then falls back to
// the GOPATH, and finally defaults to the user's home directory.
func getGoModRootPath() (string, error) {
	// Prioritize GOMODCACHE environment variable, if it is set
	modCache := os.Getenv("GOMODCACHE")
	if modCache != "" {
		return modCache, nil
	}

	// Fallback to the GOPATH if it is set
	if gopath := os.Getenv("GOPATH"); gopath != "" {
		return filepath.Join(gopath, "pkg", "mod"), nil
	}

	// If neither is set, use the default Go module cache location
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get user home directory: %w", err)
	}

	return filepath.Join(homeDir, "go", "pkg", "mod"), nil
}

// findGoModuleRoot finds the root directory of the Go module for the specified
// version of the Espresso Go SDK.
func findGoModuleRoot(version string) (string, error) {
	modCache, err := getGoModRootPath()
	if err != nil {
		return "", fmt.Errorf("failed to get Go module root path: %w", err)
	}

	modulePath, err := module.EscapePath(espressoModule)
	if err != nil {
		return "", fmt.Errorf("failed to escape module path: %w", err)
	}
	canonicalVersion := module.CanonicalVersion(version)
	installPath := fmt.Sprintf("%s@%s", modulePath, canonicalVersion)

	return filepath.Join(modCache, installPath), nil
}

// getFileDir returns the go mod catch directory destination for the specified
// version of the Espresso Go SDK.
func getFileDir(version string) string {
	path, err := findGoModuleRoot(version)
	if err != nil {
		panic(fmt.Sprintf("failed to find Go module root: %s", err))
	}

	// Let's check to make sure this directory exists

	dir, err := os.Stat(path)
	if err != nil {
		panic(fmt.Sprintf("failed to stat path %s: %s", path, err))
	}

	if !dir.IsDir() {
		panic(fmt.Sprintf("path %s is not a directory", path))
	}

	return filepath.Join(path, targetLib)
}

// FetchLatestGoSDKTag fetches the latest Go SDK release tag from GitHub.
func FetchLatestGoSDKTag() (string, error) {
	resp, err := http.Get("https://api.github.com/repos/EspressoSystems/espresso-network/releases")
	if err != nil {
		return "", fmt.Errorf("failed to fetch releases: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read body: %w", err)
	}

	var releases []map[string]interface{}
	if err := json.Unmarshal(body, &releases); err != nil {
		return "", fmt.Errorf("failed to parse JSON: %w", err)
	}

	re := regexp.MustCompile(`sdks/go/v[0-9.]*`)
	for _, release := range releases {
		if tag, ok := release["tag_name"].(string); ok {
			if re.MatchString(tag) {
				return re.FindString(tag), nil
			}
		}
	}

	return "", errors.New("could not fetch latest Espresso Go SDK release tag")
}
