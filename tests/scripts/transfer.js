const hre = require("hardhat");

async function main() {
    const sybilAddress = process.env.SYBIL_ADDRESS;
    if (!sybilAddress) {
        throw new Error("SYBIL_ADDRESS not set");
    }

    const erc20MockAddress = process.env.ERC20_MOCK_ADDRESS;
    if (!erc20MockAddress) {
        throw new Error("ERC20_MOCK_ADDRESS not set");
    }

    const Erc20Mock = await hre.ethers.getContractFactory("ERC20Mock");
    const erc20Mock = Erc20Mock.attach(erc20MockAddress);

    const provider = new ethers.providers.JsonRpcProvider();
    const address = (await provider.listAccounts())[0];
    
    await erc20Mock.mint(address, 100000000000000000000000000000000000000n);

    const txResponse = await erc20Mock.transfer(sybilAddress, 100000000000000000000000000000000000000n);
    const txReceipt = await txResponse.wait();

    console.log(txReceipt.transactionHash);
}

main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});