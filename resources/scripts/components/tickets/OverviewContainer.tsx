import React, { useEffect, useState } from 'react';
import { Button } from '@/components/elements/button';
import GreyRowBox from '@/components/elements/GreyRowBox';
import { getTickets, Ticket } from '@/api/account/tickets';
import PageContentBlock from '@/components/elements/PageContentBlock';
import NewTicketDialog from '@/components/tickets/forms/NewTicketDialog';

export default () => {
    const [visible, setVisible] = useState(false);
    const [tickets, setTickets] = useState<Ticket[]>();

    useEffect(() => {
        getTickets().then((d) => setTickets(d));
    }, []);

    return (
        <PageContentBlock
            title={'Tickets de suporte'}
            description={'Criar ou responder a um ticket de suporte.'}
            showFlashKey={'tickets'}
        >
            <NewTicketDialog open={visible} onClose={() => setVisible(false)} />
            {!tickets ? (
                <p className={'text-gray-400 text-center my-4'}>Não há tickets disponíveis.</p>
            ) : (
                <>
                    {tickets.map((ticket) => (
                        <GreyRowBox key={ticket.id}>
                            <pre>{JSON.stringify(ticket, null)}</pre>
                        </GreyRowBox>
                    ))}
                </>
            )}
            <div className={'w-full flex lg:justify-end lg:items-end'}>
                <Button onClick={() => setVisible(true)}>Criar Novo Ticket</Button>
            </div>
        </PageContentBlock>
    );
};
