// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./WrappedSOL.sol";
import "./ValidatorRegistry.sol";

contract SolanaBridge is ReentrancyGuard, Pausable, Ownable {
    WrappedSOL public wrappedSOL;
    ValidatorRegistry public validatorRegistry;

    mapping(uint256 => bool) public processedNonces;
    uint256 public feeBasisPoints;

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
        uint8 destChain,
        bytes destAddress,
        uint256 timestamp
    );

    event FeeUpdated(uint256 oldFee, uint256 newFee);
    event ValidatorRegistryUpdated(address indexed oldRegistry, address indexed newRegistry);

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

    function mintWrapped(
        address user,
        uint256 amount,
        uint256 nonce,
        bytes[] calldata signatures
    ) external nonReentrant whenNotPaused {
        require(user != address(0), "Invalid user address");
        require(amount > 0, "Amount must be > 0");
        require(!processedNonces[nonce], "Nonce already used");

        bytes32 message = keccak256(abi.encodePacked(
            user,
            amount,
            nonce,
            block.chainid,
            address(this)
        ));

        require(
            validatorRegistry.verifySignatures(message, signatures),
            "Invalid signatures"
        );

        processedNonces[nonce] = true;

        uint256 fee = (amount * feeBasisPoints) / 10000;
        uint256 amountAfterFee = amount - fee;

        wrappedSOL.mint(user, amountAfterFee);

        if (fee > 0) {
            wrappedSOL.mint(owner(), fee);
        }

        emit TokensMinted(user, amountAfterFee, nonce, block.timestamp);
    }

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

        uint256 nonce = uint256(keccak256(abi.encodePacked(
            msg.sender,
            amount,
            block.timestamp,
            block.number
        )));

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

    function updateFee(uint256 _feeBasisPoints) external onlyOwner {
        require(_feeBasisPoints <= 10000, "Fee too high (max 10000 = 100%)");
        uint256 oldFee = feeBasisPoints;
        feeBasisPoints = _feeBasisPoints;
        emit FeeUpdated(oldFee, _feeBasisPoints);
    }

    function updateValidatorRegistry(address _validatorRegistry) external onlyOwner {
        require(_validatorRegistry != address(0), "Invalid registry address");
        address oldRegistry = address(validatorRegistry);
        validatorRegistry = ValidatorRegistry(_validatorRegistry);
        emit ValidatorRegistryUpdated(oldRegistry, _validatorRegistry);
    }

    function pause() external onlyOwner {
        _pause();
    }

    function unpause() external onlyOwner {
        _unpause();
    }

    function isNonceProcessed(uint256 nonce) external view returns (bool) {
        return processedNonces[nonce];
    }

    function getFee() external view returns (uint256) {
        return feeBasisPoints;
    }
}
