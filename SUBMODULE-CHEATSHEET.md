Got it! Here's a more concise version of the **Git Submodule Cheat Sheet**:

---

# **Git Submodule Cheat Sheet**

## **Common Commands**

### Add a Submodule
```bash
git submodule add <repository-url> <submodule-path>
```

### Initialize Submodule
```bash
git submodule init
git submodule update
```

### Clone a Repo with Submodules
```bash
git clone --recurse-submodules <repository-url>
```

### Update Submodule
```bash
git submodule update --recursive --remote
```

### Check Status of Submodules
```bash
git submodule status
```

### Remove a Submodule
```bash
git submodule deinit <submodule-path>
git rm <submodule-path>
rm -rf .git/modules/<submodule-path>
```

---

## **Tips**

- To ignore submodule changes in `git status`:
  ```bash
  git config submodule.<submodule-path>.ignore all
  ```

- To perform operations in all submodules:
  ```bash
  git submodule foreach <command>
  ```

- To sync submodule references:
  ```bash
  git submodule sync
  ```

---

This should cover the basics without too much detail! Let me know if you need more specific info.
