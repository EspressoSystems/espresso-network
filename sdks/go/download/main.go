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
	"path"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"

	"github.com/spf13/cobra"
)

const targetLib = "../verification/target/lib"
const baseURL = "https://github.com/EspressoSystems/espresso-network/releases"

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
			clean()
		},
	}

	var filePath string
	var checkSum string
	var linkCmd = &cobra.Command{
		Use:   "link",
		Short: "Create a symlink to the downloaded library",
		Run: func(cmd *cobra.Command, args []string) {
			createSymlink(filePath, checkSum)
		},
	}
	linkCmd.Flags().StringVarP(&filePath, "filePath", "f", "", "Specify the file path to create the symlink in")
	linkCmd.Flags().StringVarP(&checkSum, "checkSum", "c", "", "Specify the checkSum to create the symlink in")

	rootCmd.AddCommand(downloadCmd, cleanCmd, linkCmd)
	err := rootCmd.Execute()
	if err != nil {
		fmt.Printf("Failed to execute command: %s\n", err)
		os.Exit(1)
	}
}

func createSymlink(path string, checkSum string) {
	linkName := getFileName()
	fileDir := getFileDir()
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

	if err := os.MkdirAll(fileDir, 0755); err != nil {
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
			latestTag, err := FetchLatestGoSDKTag()
			if err != nil {
				fmt.Printf("Failed to fetch latest Espresso Go SDK release tag: %s\n", err)
				os.Exit(1)
			}
			version = latestTag
			fmt.Printf("Using latest version %s to download the library\n", version)
		} else {
			if strings.HasPrefix(version, "v") {
				version = fmt.Sprintf("sdks/go/%s", version)
			}
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

func clean() {
	fileDir := getFileDir()
	err := os.RemoveAll(fileDir)
	if err != nil {
		fmt.Printf("Failed to clean this symlink: %s\n", err)
		os.Exit(1)
	}
	fmt.Println("Cleaned the symlink.")
}

func getFileName() string {
	arch := runtime.GOARCH
	os := runtime.GOOS

	var fileName string
	var extension string

	// Determine file extension based on OS
	if os == "darwin" {
		extension = ".dylib"
	} else if os == "linux" {
		extension = ".so"
	} else {
		panic(fmt.Sprintf("unsupported OS: %s", os))
	}

	// Determine architecture-specific prefix
	switch arch {
	case "amd64":
		if os == "darwin" {
			fileName = "x86_64-apple-darwin"
		} else if os == "linux" {
			fileName = "x86_64-unknown-linux-musl"
		}
	case "arm64":
		if os == "darwin" {
			fileName = "aarch64-apple-darwin"
		} else if os == "linux" {
			fileName = "aarch64-unknown-linux-musl"
		}
	default:
		panic(fmt.Sprintf("unsupported architecture: %s", arch))
	}

	if fileName == "" {
		panic(fmt.Sprintf("unsupported os: %s", os))
	}

	return fmt.Sprintf("libespresso_crypto_helper-%s%s", fileName, extension)
}

func getFileDir() string {
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		panic("No caller information")
	}

	return filepath.Join(path.Dir(filename), targetLib)
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
