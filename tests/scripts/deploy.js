const hre = require("hardhat");

async function main() {
    const Erc20Mock = await hre.ethers.getContractFactory("ERC20Mock");
    let erc20Mock = await Erc20Mock.deploy("Mock", "MOCK");
    erc20Mock = await erc20Mock.deployed();

    console.log(erc20Mock.address);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
