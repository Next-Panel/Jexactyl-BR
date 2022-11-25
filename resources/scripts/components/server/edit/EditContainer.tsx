import tw from 'twin.macro';
import { breakpoint } from '@/theme';
import * as Icon from 'react-feather';
import React, { useState } from 'react';
import useFlash from '@/plugins/useFlash';
import styled from 'styled-components/macro';
import { ServerContext } from '@/state/server';
import editServer from '@/api/server/editServer';
import { Dialog } from '@/components/elements/dialog';
import { Button } from '@/components/elements/button/index';
import TitledGreyBox from '@/components/elements/TitledGreyBox';
import FlashMessageRender from '@/components/FlashMessageRender';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import ServerContentBlock from '@/components/elements/ServerContentBlock';

const Container = styled.div`
    ${tw`flex flex-wrap`};

    & > div {
        ${tw`w-full`};

        ${breakpoint('sm')`
      width: calc(50% - 1rem);
    `}

        ${breakpoint('md')`
      ${tw`w-auto flex-1`};
    `}
    }
`;

const Wrapper = styled.div`
    ${tw`text-2xl flex flex-row justify-center items-center`};
`;

export default () => {
    const [submitting, setSubmitting] = useState(false);
    const [resource, setResource] = useState('');
    const [amount, setAmount] = useState(0);

    const uuid = ServerContext.useStoreState((state) => state.server.data!.uuid);
    const { clearFlashes, addFlash, clearAndAddHttpError } = useFlash();

    const edit = (resource: string, amount: number) => {
        clearFlashes('server:edit');
        setSubmitting(true);

        editServer(uuid, resource, amount)
            .then(() => {
                setSubmitting(false);
                addFlash({
                    key: 'server:edit',
                    type: 'success',
                    message: 'Os recursos do servidor foram editados com sucesso.',
                });
            })
            .catch((error) => clearAndAddHttpError({ key: 'server:edit', error }));
    };

    return (
        <ServerContentBlock title={'Editar servidor'}>
            <SpinnerOverlay size={'large'} visible={submitting} />
            <Dialog.Confirm
                open={submitting}
                onClose={() => setSubmitting(false)}
                title={'Confirme a edição de recursos'}
                onConfirmed={() => edit(resource, amount)}
            >
                Isso removerá os recursos da sua conta e os adicionará ao seu servidor. Você tem certeza que quer
                Prosseguir?
            </Dialog.Confirm>
            <FlashMessageRender byKey={'server:edit'} css={tw`mb-4`} />
            <h1 className={'j-left text-5xl'}>Edit Resources</h1>
            <h3 className={'j-left text-2xl mt-2 text-neutral-500 mb-10'}>
                Adicione e remova os recursos do seu servidor.
            </h3>
            <Container css={tw`lg:grid lg:grid-cols-3 gap-4 my-10`}>
                <TitledGreyBox title={'Editar limite de CPU do servidor'} css={tw`mt-8 sm:mt-0`}>
                    <Wrapper>
                        <Icon.Cpu size={40} />
                        <Button.Success
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('cpu');
                                setAmount(50);
                            }}
                        >
                            <Icon.Plus />
                        </Button.Success>
                        <Button.Danger
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('cpu');
                                setAmount(-50);
                            }}
                        >
                            <Icon.Minus />
                        </Button.Danger>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Altere a quantidade de CPU atribuída ao servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>O limite não pode ser inferior a 50%.</p>
                </TitledGreyBox>
                <TitledGreyBox title={'Editar Limite de RAM do servidor'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.PieChart size={40} />
                        <Button.Success
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('memory');
                                setAmount(1024);
                            }}
                        >
                            <Icon.Plus />
                        </Button.Success>
                        <Button.Danger
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('memory');
                                setAmount(-1024);
                            }}
                        >
                            <Icon.Minus />
                        </Button.Danger>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Altere a quantidade de RAM atribuída ao servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>O limite não pode ser menor que 1 GB.</p>
                </TitledGreyBox>
                <TitledGreyBox title={'Editar limite de armazenamento do servidor'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.HardDrive size={40} />
                        <Button.Success
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('disk');
                                setAmount(1024);
                            }}
                        >
                            <Icon.Plus />
                        </Button.Success>
                        <Button.Danger
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('disk');
                                setAmount(-1024);
                            }}
                        >
                            <Icon.Minus />
                        </Button.Danger>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Altere a quantidade de armazenamento atribuída ao servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>O limite não pode ser menor que 1 GB.</p>
                </TitledGreyBox>
                <TitledGreyBox title={'Editar Quantidade de portas do servidor'} css={tw`mt-8 sm:mt-0`}>
                    <Wrapper>
                        <Icon.Share2 size={40} />
                        <Button.Success
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('allocation_limit');
                                setAmount(1);
                            }}
                        >
                            <Icon.Plus />
                        </Button.Success>
                        <Button.Danger
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('allocation_limit');
                                setAmount(-1);
                            }}
                        >
                            <Icon.Minus />
                        </Button.Danger>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                       Altere o limite das portas atribuídas ao servidor.
                    </p>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>O limite não pode ser menor que 1.</p>
                </TitledGreyBox>
                <TitledGreyBox title={'Editar limite de backup do servidor'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.Archive size={40} />
                        <Button.Success
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('backup_limit');
                                setAmount(1);
                            }}
                        >
                            <Icon.Plus />
                        </Button.Success>
                        <Button.Danger
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('backup_limit');
                                setAmount(-1);
                            }}
                        >
                            <Icon.Minus />
                        </Button.Danger>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Altere o limite de backups atribuídos ao servidor.
                    </p>
                </TitledGreyBox>
                <TitledGreyBox title={'Editar limite de banco de dados do servidor'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <Wrapper>
                        <Icon.Database size={40} />
                        <Button.Success
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('database_limit');
                                setAmount(1);
                            }}
                        >
                            <Icon.Plus />
                        </Button.Success>
                        <Button.Danger
                            css={tw`ml-4`}
                            onClick={() => {
                                setSubmitting(true);
                                setResource('database_limit');
                                setAmount(-1);
                            }}
                        >
                            <Icon.Minus />
                        </Button.Danger>
                    </Wrapper>
                    <p css={tw`mt-1 text-gray-500 text-xs flex justify-center`}>
                        Altere o limite dos bancos de dados atribuídos ao servidor.
                    </p>
                </TitledGreyBox>
            </Container>
        </ServerContentBlock>
    );
};
