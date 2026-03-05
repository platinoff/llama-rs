# GitHub setup for llama.rs

## Create the repository

1. Open: https://github.com/new?name=llama-rs&description=Ultra-fast+and+safe+Rust+wrapper+around+llama.cpp
2. Set **Repository name** to `llama-rs`.
3. Choose **Public**, leave "Add a README" **unchecked** (we already have one).
4. Click **Create repository**.

## Push from local

From the project root:

```powershell
cd s:\rust\llama-rs\llama-rs-project
git remote -v   # should show origin → https://github.com/platinoff/llama-rs.git
git push -u origin master
```

If your GitHub default branch is `main` and you prefer it:

```powershell
git branch -M main
git push -u origin main
```

Use your **Personal Access Token** (gittoken) as the password when prompted.

If the repo was created under a different GitHub username, fix the remote:

```powershell
git remote set-url origin https://github.com/YOUR_GITHUB_USERNAME/llama-rs.git
```
