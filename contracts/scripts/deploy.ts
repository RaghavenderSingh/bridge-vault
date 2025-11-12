import { ethers } from "hardhat";

async function main() {
  const [deployer] = await ethers.getSigners();

  console.log("\n🚀 Starting deployment...");
  console.log("═══════════════════════════════════════════════════════════");
  console.log("Deploying contracts with account:", deployer.address);
  console.log("Account balance:", ethers.formatEther(await ethers.provider.getBalance(deployer.address)), "ETH");
  console.log("═══════════════════════════════════════════════════════════\n");

  // Get network info
  const network = await ethers.provider.getNetwork();
  console.log("Network:", network.name);
  console.log("Chain ID:", network.chainId.toString());
  console.log("\n");

  // Deploy WrappedSOL
  console.log("1️⃣  Deploying WrappedSOL...");
  const WrappedSOL = await ethers.getContractFactory("WrappedSOL");
  const wrappedSOL = await WrappedSOL.deploy();
  await wrappedSOL.waitForDeployment();
  const wrappedSOLAddress = await wrappedSOL.getAddress();
  console.log("✅ WrappedSOL deployed to:", wrappedSOLAddress);
  console.log("\n");

  // Deploy ValidatorRegistry
  console.log("2️⃣  Deploying ValidatorRegistry...");
  console.log("⚠️  Using single validator for testing - REPLACE WITH REAL VALIDATORS FOR PRODUCTION!");

  // For testing: using single validator with threshold of 1
  // TODO: Replace with real validator addresses for production
  const validators = [
    deployer.address, // Validator 1 (deployer address for testing)
  ];

  const threshold = 1; // 1-of-1 for testing (use 2-of-3 or higher for production)

  console.log("Validators:", validators);
  console.log("Threshold:", threshold);

  const ValidatorRegistry = await ethers.getContractFactory("ValidatorRegistry");
  const validatorRegistry = await ValidatorRegistry.deploy(validators, threshold);
  await validatorRegistry.waitForDeployment();
  const validatorRegistryAddress = await validatorRegistry.getAddress();
  console.log("✅ ValidatorRegistry deployed to:", validatorRegistryAddress);
  console.log("\n");

  // Deploy SolanaBridge
  console.log("3️⃣  Deploying SolanaBridge...");
  const feeBasisPoints = 50; // 0.5% fee
  console.log("Fee:", feeBasisPoints / 100 + "%");

  const SolanaBridge = await ethers.getContractFactory("SolanaBridge");
  const bridge = await SolanaBridge.deploy(
    wrappedSOLAddress,
    validatorRegistryAddress,
    feeBasisPoints
  );
  await bridge.waitForDeployment();
  const bridgeAddress = await bridge.getAddress();
  console.log("✅ SolanaBridge deployed to:", bridgeAddress);
  console.log("\n");

  // Configure WrappedSOL
  console.log("4️⃣  Configuring WrappedSOL...");
  const tx = await wrappedSOL.setBridge(bridgeAddress);
  await tx.wait();
  console.log("✅ Bridge set in WrappedSOL");
  console.log("Transaction hash:", tx.hash);
  console.log("\n");

  // Summary
  console.log("═══════════════════════════════════════════════════════════");
  console.log("🎉 Deployment Complete!");
  console.log("═══════════════════════════════════════════════════════════");
  console.log("\nDeployed Contracts:");
  console.log("───────────────────────────────────────────────────────────");
  console.log("WrappedSOL:         ", wrappedSOLAddress);
  console.log("ValidatorRegistry:  ", validatorRegistryAddress);
  console.log("SolanaBridge:       ", bridgeAddress);
  console.log("───────────────────────────────────────────────────────────");

  // Create deployment info object
  const deploymentInfo = {
    network: network.name,
    chainId: network.chainId.toString(),
    deployer: deployer.address,
    timestamp: new Date().toISOString(),
    contracts: {
      wrappedSOL: wrappedSOLAddress,
      validatorRegistry: validatorRegistryAddress,
      bridge: bridgeAddress,
    },
    config: {
      validators,
      threshold,
      feeBasisPoints,
    },
  };

  console.log("\n📋 Deployment Info (JSON):");
  console.log(JSON.stringify(deploymentInfo, null, 2));

  // Verify instructions
  console.log("\n");
  console.log("═══════════════════════════════════════════════════════════");
  console.log("📝 Next Steps:");
  console.log("═══════════════════════════════════════════════════════════");
  console.log("\n1. Verify contracts on Etherscan:");
  console.log("   npx hardhat verify --network sepolia", wrappedSOLAddress);
  console.log("   npx hardhat verify --network sepolia", validatorRegistryAddress, `'["${validators[0]}","${validators[1]}","${validators[2]}"]'`, threshold);
  console.log("   npx hardhat verify --network sepolia", bridgeAddress, wrappedSOLAddress, validatorRegistryAddress, feeBasisPoints);

  console.log("\n2. Update validator addresses in ValidatorRegistry if needed");
  console.log("   (if you used placeholder addresses)");

  console.log("\n3. Configure your relayer with these contract addresses");

  console.log("\n4. Test the bridge:");
  console.log("   - Try minting tokens with valid signatures");
  console.log("   - Try burning tokens to bridge back");

  console.log("\n5. Monitor the bridge:");
  console.log("   - Watch for TokensMinted events");
  console.log("   - Watch for TokensBurned events");

  console.log("\n═══════════════════════════════════════════════════════════\n");

  // Save deployment info to file
  const fs = require("fs");
  const deploymentFile = `deployment-${network.name}-${Date.now()}.json`;
  fs.writeFileSync(deploymentFile, JSON.stringify(deploymentInfo, null, 2));
  console.log("💾 Deployment info saved to:", deploymentFile);
  console.log("\n");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error("\n❌ Deployment failed:");
    console.error(error);
    process.exitCode = 1;
  });
