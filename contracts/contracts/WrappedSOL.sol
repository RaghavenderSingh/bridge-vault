// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title WrappedSOL
 * @notice ERC-20 token representing SOL on Ethereum
 * @dev Only the bridge contract can mint and burn tokens
 */
contract WrappedSOL is ERC20, Ownable {
    address public bridge;

    event BridgeUpdated(address indexed oldBridge, address indexed newBridge);

    /**
     * @notice Constructor initializes the token with name and symbol
     */
    constructor() ERC20("Wrapped Solana", "wSOL") Ownable(msg.sender) {}

    /**
     * @notice Set the bridge contract address
     * @param _bridge Address of the bridge contract
     * @dev Only owner can set the bridge address
     */
    function setBridge(address _bridge) external onlyOwner {
        require(_bridge != address(0), "Invalid bridge address");
        address oldBridge = bridge;
        bridge = _bridge;
        emit BridgeUpdated(oldBridge, _bridge);
    }

    /**
     * @notice Mint tokens to an address
     * @param to Recipient address
     * @param amount Amount to mint
     * @dev Only bridge can mint
     */
    function mint(address to, uint256 amount) external {
        require(msg.sender == bridge, "Only bridge can mint");
        require(to != address(0), "Cannot mint to zero address");
        _mint(to, amount);
    }

    /**
     * @notice Burn tokens from an address
     * @param from Address to burn from
     * @param amount Amount to burn
     * @dev Only bridge can burn
     */
    function burnFrom(address from, uint256 amount) external {
        require(msg.sender == bridge, "Only bridge can burn");
        _burn(from, amount);
    }

    /**
     * @notice Get token decimals (18 to match SOL precision)
     */
    function decimals() public pure override returns (uint8) {
        return 18;
    }
}
