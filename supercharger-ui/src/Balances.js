import React, { useEffect, useState } from 'react';
import { Table } from 'semantic-ui-react';

export default function Balances (props) {
  const { api, keyring } = props;
  let accounts = keyring.getPairs();
  accounts = [accounts[0], accounts[3]];
  const addresses = accounts.map(account => account.address);
  const accountNames = accounts.map((account) => account.meta.name);
  const [balances, setBalances] = useState({});

  useEffect(() => {
    let unsubscribeAll


    api.query.fungible.balances( [0,  accounts[0].address], (balance) => console.log("hack", balance.toString()))
    api.query.fungible.balances( [0,  accounts[1].address], (balance) => console.log("hack", balance.toString()))

      // .multi(addresses, (currentBalances) => {
      //   const balancesMap = addresses.reduce((acc, address, index) => ({
      //     ...acc,
      //     [address]: currentBalances[index].toString()
      //   }), {});
      //   setBalances(balancesMap);
      // })
      .then(unsub => { unsubscribeAll = unsub; })
      .catch(console.error);

    return () => unsubscribeAll && unsubscribeAll();
  }, [addresses, api.query.balances.freeBalance, setBalances]);

  return (
    <>
      <h1>Balances</h1>
      <Table celled striped>
        <Table.Body>
          {addresses.map((address, index) => {
            return (
              <Table.Row key={index}>
                <Table.Cell textAlign='right'>{accountNames[index]}</Table.Cell>
                <Table.Cell>{address}</Table.Cell>
                <Table.Cell>{balances && balances[address]}</Table.Cell>
              </Table.Row>
            );
          })}
        </Table.Body>
      </Table>
    </>
  );
}
