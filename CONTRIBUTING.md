# Contributing to Bridge Vault

Thank you for your interest in contributing to Bridge Vault. This document provides guidelines and instructions for contributing to the project.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [How to Contribute](#how-to-contribute)
4. [Development Workflow](#development-workflow)
5. [Coding Standards](#coding-standards)
6. [Testing Guidelines](#testing-guidelines)
7. [Pull Request Process](#pull-request-process)
8. [Issue Reporting](#issue-reporting)

## Code of Conduct

### Our Standards

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Prioritize the project's goals and community health
- Accept responsibility and learn from mistakes

### Unacceptable Behavior

- Harassment or discriminatory language
- Trolling or personal attacks
- Publishing others' private information
- Spam or off-topic discussions
- Any conduct that could be considered unprofessional

## Getting Started

### Prerequisites

Before contributing, ensure you have:

1. Read the [README](README.md) and [SETUP guide](SETUP.md)
2. Set up your development environment
3. Familiarized yourself with the codebase structure
4. Reviewed existing issues and pull requests

### First-Time Contributors

If this is your first contribution:

1. Look for issues labeled `good-first-issue` or `help-wanted`
2. Comment on the issue expressing your interest
3. Wait for approval from maintainers before starting work
4. Ask questions if anything is unclear

## How to Contribute

### Types of Contributions We Need

#### Code Contributions
- Bug fixes
- New features
- Performance improvements
- Gas optimizations for smart contracts
- Security enhancements

#### Documentation
- Improving existing documentation
- Adding code examples
- Writing tutorials
- Translating documentation

#### Testing
- Writing unit tests
- Integration tests
- End-to-end test scenarios
- Security testing

#### Design
- UI/UX improvements for the web app
- Creating diagrams and visual aids
- Improving user flows

#### Community
- Answering questions in issues
- Reviewing pull requests
- Helping others get started
- Sharing the project

## Development Workflow

### 1. Fork and Clone

```bash
# Fork the repository on GitHub, then:
git clone https://github.com/YOUR_USERNAME/bridge-vault.git
cd bridge-vault

# Add upstream remote
git remote add upstream https://github.com/RaghavenderSingh/bridge-vault.git
```

### 2. Create a Branch

```bash
# Update your main branch
git checkout main
git pull upstream main

# Create a new branch
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

Branch naming conventions:
- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `test/description` - Test additions or modifications
- `refactor/description` - Code refactoring

### 3. Make Changes

- Write clean, readable code
- Follow existing code style
- Add comments for complex logic
- Update documentation as needed
- Write or update tests

### 4. Commit Your Changes

```bash
git add .
git commit -m "type: clear description of changes"
```

Commit message format:
```
type: subject line (max 50 characters)

Optional body explaining what and why (wrap at 72 characters)

Fixes #issue_number
```

Commit types:
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Adding or updating tests
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `chore:` Maintenance tasks

Examples:
```
feat: add multi-signature validation to bridge contract

fix: prevent duplicate transaction processing in relayer

docs: update setup guide with troubleshooting steps

test: add integration tests for cross-chain transfers
```

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a pull request on GitHub.

## Coding Standards

### General Guidelines

- Write self-documenting code with clear variable and function names
- Keep functions small and focused on a single responsibility
- Avoid premature optimization
- Handle errors appropriately
- Never commit sensitive information (keys, passwords, etc.)

### Rust Code (Solana Program & Relayer)

```rust
// Use descriptive names
fn process_bridge_transfer(amount: u64, recipient: Pubkey) -> Result<()> {
    // Validate inputs
    require!(amount > 0, BridgeError::InvalidAmount);

    // Clear logic flow
    let fee = calculate_fee(amount)?;
    let net_amount = amount.checked_sub(fee)
        .ok_or(BridgeError::Overflow)?;

    // Process transfer
    transfer_tokens(recipient, net_amount)?;

    Ok(())
}
```

Standards:
- Follow Rust naming conventions
- Use `Result` and `Option` types appropriately
- Implement proper error handling
- Add documentation comments for public APIs
- Run `cargo fmt` and `cargo clippy` before committing

### Solidity Code (Ethereum Contracts)

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title BridgeFunction
 * @notice Handles cross-chain token transfers
 * @dev Implements multi-signature validation
 */
function processTransfer(
    uint256 amount,
    address recipient,
    bytes[] calldata signatures
) external nonReentrant whenNotPaused {
    require(amount > 0, "Invalid amount");
    require(recipient != address(0), "Invalid recipient");
    require(signatures.length >= minSignatures, "Insufficient signatures");

    // Function logic
}
```

Standards:
- Follow Solidity style guide
- Use latest stable compiler version
- Implement security best practices (checks-effects-interactions)
- Add NatSpec comments
- Use OpenZeppelin contracts where applicable
- Optimize for gas efficiency

### TypeScript/JavaScript (Web App)

```typescript
// Use TypeScript types
interface BridgeTransfer {
  amount: number;
  recipient: string;
  sourceChain: 'solana' | 'ethereum';
  destinationChain: 'solana' | 'ethereum';
}

// Clear function signatures
async function initiateBridge(transfer: BridgeTransfer): Promise<string> {
  // Validate input
  if (transfer.amount <= 0) {
    throw new Error('Amount must be positive');
  }

  // Process transfer
  const txHash = await submitTransaction(transfer);
  return txHash;
}
```

Standards:
- Use TypeScript for type safety
- Follow ESLint configuration
- Use async/await over promises
- Handle errors gracefully
- Use functional components with hooks in React

## Testing Guidelines

### Required Tests

All code contributions should include tests:

#### Solana Program Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_transfer() {
        // Setup
        let amount = 1_000_000;

        // Execute
        let result = process_transfer(amount);

        // Assert
        assert!(result.is_ok());
    }
}
```

Run tests:
```bash
cd programs/bridge-vault
cargo test-sbf
```

#### Ethereum Contract Tests
```typescript
describe("SolanaBridge", function () {
  it("Should process valid bridge transfers", async function () {
    const amount = ethers.parseEther("1.0");

    await expect(bridge.mintWrapped(amount, nonce, signatures))
      .to.emit(bridge, "TokensMinted")
      .withArgs(user.address, amount, nonce);
  });
});
```

Run tests:
```bash
cd contracts
npx hardhat test
```

#### Integration Tests

Test cross-chain scenarios:
- Full bridge flow from Solana to Ethereum
- Reverse bridge from Ethereum to Solana
- Error handling and recovery
- Edge cases

### Test Coverage

- Aim for at least 80% code coverage
- Test both success and failure cases
- Include edge cases and boundary conditions
- Test with realistic scenarios

## Pull Request Process

### Before Submitting

- [ ] Code follows project style guidelines
- [ ] All tests pass locally
- [ ] New tests added for new functionality
- [ ] Documentation updated if needed
- [ ] No merge conflicts with main branch
- [ ] Commits are clean and well-described

### PR Description Template

```markdown
## Description
Brief description of what this PR does

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Related Issue
Fixes #(issue number)

## Changes Made
- Change 1
- Change 2
- Change 3

## Testing
Describe how you tested these changes

## Screenshots (if applicable)
Add screenshots for UI changes

## Checklist
- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have updated the documentation accordingly
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
```

### Review Process

1. Maintainers will review your PR within a few days
2. Address any feedback or requested changes
3. Once approved, a maintainer will merge your PR
4. Your contribution will be credited in the release notes

### After Your PR is Merged

- Delete your branch
- Update your local repository
- Celebrate your contribution!

## Issue Reporting

### Bug Reports

When reporting bugs, include:

```markdown
**Description**
Clear description of the bug

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. See error

**Expected Behavior**
What you expected to happen

**Actual Behavior**
What actually happened

**Environment**
- OS: [e.g., macOS, Ubuntu]
- Rust version: [e.g., 1.70]
- Solana CLI version: [e.g., 1.17]
- Node version: [e.g., 18.0]

**Logs/Screenshots**
Add any relevant logs or screenshots
```

### Feature Requests

When proposing features:

```markdown
**Problem Statement**
What problem does this feature solve?

**Proposed Solution**
How should this feature work?

**Alternatives Considered**
What other approaches did you consider?

**Additional Context**
Any other relevant information
```

## Development Tips

### Useful Commands

```bash
# Run all tests
npm run test:all

# Format code
cargo fmt
npm run format

# Lint code
cargo clippy
npm run lint

# Check for security issues
cargo audit
npm audit

# Build all components
npm run build:all
```

### Debugging

- Use `console.log` / `msg!` / `println!` appropriately
- Check relayer logs for cross-chain issues
- Use Solana Explorer and Etherscan for transaction debugging
- Test on devnet/testnet before mainnet

### Getting Help

- Check existing documentation
- Search closed issues for similar problems
- Ask in GitHub Discussions
- Reach out on X: [@itsrsc_](https://x.com/itsrsc_)

## Recognition

Contributors will be:
- Listed in release notes
- Credited in the README (for significant contributions)
- Given a shout-out on social media
- Eligible for future bounties and rewards (when available)

## License

By contributing to Bridge Vault, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Bridge Vault! Your efforts help make cross-chain interoperability accessible to everyone.
