// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./WrappedSOL.sol";
import "./ValidatorRegistry.sol";

/**
 * @title SolanaBridge
 * @notice Main bridge contract for minting and burning wrapped SOL tokens
 * @dev Implements multi-signature validation, fee mechanism, and security features
 */
contract SolanaBridge is ReentrancyGuard, Pausable, Ownable {
    WrappedSOL public wrappedSOL;
    ValidatorRegistry public validatorRegistry;

    mapping(uint256 => bool) public processedNonces;
    uint256 public feeBasisPoints; // Fee in basis points (50 = 0.5%, 100 = 1%)

    event TokensMinted(
        address indexed user,
        uint256 amount,
        uint256 nonce,
        uint256 timestamp
    );

    event TokensBurned(
        address indexed user,
        uint256 amount,
        uint256 nonce,
        uint8 destChain, // 0=Solana, 2=Sui
        bytes destAddress,
        uint256 timestamp
    );

    event FeeUpdated(uint256 oldFee, uint256 newFee);
    event ValidatorRegistryUpdated(address indexed oldRegistry, address indexed newRegistry);

    /**
     * @notice Constructor initializes the bridge with required contracts
     * @param _wrappedSOL Address of the WrappedSOL token contract
     * @param _validatorRegistry Address of the ValidatorRegistry contract
     * @param _feeBasisPoints Initial fee in basis points (max 10000 = 100%)
     */
    constructor(
        address _wrappedSOL,
        address _validatorRegistry,
        uint256 _feeBasisPoints
    ) Ownable(msg.sender) {
        require(_wrappedSOL != address(0), "Invalid wSOL address");
        require(_validatorRegistry != address(0), "Invalid registry address");
        require(_feeBasisPoints <= 10000, "Fee too high");

        wrappedSOL = WrappedSOL(_wrappedSOL);
        validatorRegistry = ValidatorRegistry(_validatorRegistry);
        feeBasisPoints = _feeBasisPoints;
    }

    /**
     * @notice Mint wrapped SOL tokens (called by relayer with validator signatures)
     * @param user Recipient address
     * @param amount Amount to mint (in wei, 18 decimals)
     * @param nonce Unique nonce from Solana bridge
     * @param signatures Array of validator signatures
     * @dev Requires valid multi-sig, prevents replay attacks via nonce
     */
    function mintWrapped(
        address user,
        uint256 amount,
        uint256 nonce,
        bytes[] calldata signatures
    ) external nonReentrant whenNotPaused {
        require(user != address(0), "Invalid user address");
        require(amount > 0, "Amount must be > 0");
        require(!processedNonces[nonce], "Nonce already used");

        // Construct message to verify
        bytes32 message = keccak256(abi.encodePacked(
            user,
            amount,
            nonce,
            block.chainid,
            address(this)
        ));

        // Verify validator signatures
        require(
            validatorRegistry.verifySignatures(message, signatures),
            "Invalid signatures"
        );

        // Mark nonce as processed (prevents replay attacks)
        processedNonces[nonce] = true;

        // Calculate fee
        uint256 fee = (amount * feeBasisPoints) / 10000;
        uint256 amountAfterFee = amount - fee;

        // Mint tokens to user
        wrappedSOL.mint(user, amountAfterFee);

        // Mint fee to owner
        if (fee > 0) {
            wrappedSOL.mint(owner(), fee);
        }

        emit TokensMinted(user, amountAfterFee, nonce, block.timestamp);
    }

    /**
     * @notice Burn wrapped SOL to bridge back to Solana/Sui
     * @param amount Amount to burn
     * @param destChain Destination chain (0=Solana, 2=Sui)
     * @param destAddress Destination address (32 bytes for Solana/Sui, 20 for Ethereum)
     * @dev Burns tokens and emits event for relayer to process
     */
    function burnAndBridge(
        uint256 amount,
        uint8 destChain,
        bytes calldata destAddress
    ) external nonReentrant whenNotPaused {
        require(amount > 0, "Amount must be > 0");
        require(destChain == 0 || destChain == 2, "Invalid chain (0=Solana, 2=Sui)");
        require(
            destAddress.length == 32 || destAddress.length == 20,
            "Invalid address length"
        );

        // Generate unique nonce for this burn
        uint256 nonce = uint256(keccak256(abi.encodePacked(
            msg.sender,
            amount,
            block.timestamp,
            block.number
        )));

        // Burn tokens from user
        wrappedSOL.burnFrom(msg.sender, amount);

        emit TokensBurned(
            msg.sender,
            amount,
            nonce,
            destChain,
            destAddress,
            block.timestamp
        );
    }

    /**
     * @notice Update the fee percentage
     * @param _feeBasisPoints New fee in basis points (max 10000)
     * @dev Only owner can update fees
     */
    function updateFee(uint256 _feeBasisPoints) external onlyOwner {
        require(_feeBasisPoints <= 10000, "Fee too high (max 10000 = 100%)");
        uint256 oldFee = feeBasisPoints;
        feeBasisPoints = _feeBasisPoints;
        emit FeeUpdated(oldFee, _feeBasisPoints);
    }

    /**
     * @notice Update the validator registry contract
     * @param _validatorRegistry New validator registry address
     * @dev Only owner can update the registry
     */
    function updateValidatorRegistry(address _validatorRegistry) external onlyOwner {
        require(_validatorRegistry != address(0), "Invalid registry address");
        address oldRegistry = address(validatorRegistry);
        validatorRegistry = ValidatorRegistry(_validatorRegistry);
        emit ValidatorRegistryUpdated(oldRegistry, _validatorRegistry);
    }

    /**
     * @notice Pause the contract (emergency stop)
     * @dev Only owner can pause
     */
    function pause() external onlyOwner {
        _pause();
    }

    /**
     * @notice Unpause the contract
     * @dev Only owner can unpause
     */
    function unpause() external onlyOwner {
        _unpause();
    }

    /**
     * @notice Check if a nonce has been processed
     * @param nonce The nonce to check
     * @return bool True if the nonce has been used
     */
    function isNonceProcessed(uint256 nonce) external view returns (bool) {
        return processedNonces[nonce];
    }

    /**
     * @notice Get the current fee in basis points
     * @return uint256 Fee in basis points
     */
    function getFee() external view returns (uint256) {
        return feeBasisPoints;
    }
}
