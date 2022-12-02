import tw from 'twin.macro';
import React, { useState } from 'react';
import useFlash from '@/plugins/useFlash';
import { useStoreState } from '@/state/hooks';
import Code from '@/components/elements/Code';
import { ServerContext } from '@/state/server';
import Input from '@/components/elements/Input';
import renewServer from '@/api/server/renewServer';
import deleteServer from '@/api/server/deleteServer';
import { Button } from '@/components/elements/button';
import { Dialog } from '@/components/elements/dialog';
import ServerErrorSvg from '@/assets/images/server_error.svg';
import FlashMessageRender from '@/components/FlashMessageRender';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import PageContentBlock from '@/components/elements/PageContentBlock';

export default () => {
    const [name, setName] = useState('');

    const [isSubmit, setSubmit] = useState(false);
    const [renewDialog, setRenewDialog] = useState(false);
    const [deleteDialog, setDeleteDialog] = useState(false);
    const [confirmDialog, setConfirmDialog] = useState(false);

    const { clearFlashes, clearAndAddHttpError } = useFlash();
    const store = useStoreState((state) => state.storefront.data!);
    const uuid = ServerContext.useStoreState((state) => state.server.data!.uuid);
    const serverName = ServerContext.useStoreState((state) => state.server.data!.name);
    const renewable = ServerContext.useStoreState((state) => state.server.data?.renewable);

    const doRenewal = () => {
        clearFlashes('server:renewal');
        setSubmit(true);

        renewServer(uuid)
            .then(() => {
                setSubmit(false);
                // @ts-expect-error this is valid
                window.location = '/';
            })
            .catch((error) => {
                clearAndAddHttpError({ key: 'server:renewal', error });
                setSubmit(false);
            });
    };

    const doDeletion = (e: React.FormEvent<HTMLFormElement>) => {
        e.preventDefault();
        e.stopPropagation();

        clearFlashes('server:renewal');
        setSubmit(true);

        deleteServer(uuid, name)
            .then(() => {
                setSubmit(false);
                // @ts-expect-error this is valid
                window.location = '/store';
            })
            .catch((error) => {
                clearAndAddHttpError({ key: 'server:renewal', error });
                setSubmit(false);
            });
    };

    return (
        <>
            <Dialog.Confirm
                open={renewDialog}
                onClose={() => setRenewDialog(false)}
                title={'Confirm server renewal'}
                confirm={'Continue'}
                onConfirmed={() => doRenewal()}
            >
                <SpinnerOverlay visible={isSubmit} />
                Tem certeza que deseja gastar {store.renewals.cost} {store.currency} Para renovar seu servidor?
            </Dialog.Confirm>
            <Dialog.Confirm
                open={deleteDialog}
                onClose={() => setDeleteDialog(false)}
                title={'Confirm server deletion'}
                confirm={'Continue'}
                onConfirmed={() => setConfirmDialog(true)}
            >
                <SpinnerOverlay visible={isSubmit} />
                Essa ação removerá seu servidor do sistema, juntamente com todos os arquivos e configurações.
            </Dialog.Confirm>
            <form id={'delete-suspended-server-form'} onSubmit={doDeletion}>
                <Dialog open={confirmDialog} title={'Confirm server deletion'} onClose={() => setConfirmDialog(false)}>
                    {name !== serverName && (
                        <>
                            <p className={'my-2 text-gray-400'}>
                                Modelo <Code>{serverName}</Code> abaixo de.
                            </p>
                            <Input type={'text'} value={name} onChange={(n) => setName(n.target.value)} />
                        </>
                    )}
                    <Button
                        disabled={name !== serverName}
                        type={'submit'}
                        className={'mt-2'}
                        form={'delete-suspended-server-form'}
                    >
                        Confirme
                    </Button>
                </Dialog>
            </form>
            <PageContentBlock title={'Server Suspended'}>
                <FlashMessageRender byKey={'server:renewal'} css={tw`mb-1`} />
                <div css={tw`flex justify-center`}>
                    <div
                        css={tw`w-full sm:w-3/4 md:w-1/2 p-12 md:p-20 bg-neutral-900 rounded-lg shadow-lg text-center relative`}
                    >
                        <img src={ServerErrorSvg} css={tw`w-2/3 h-auto select-none mx-auto`} />
                        <h2 css={tw`mt-10 font-bold text-4xl`}>Suspenso</h2>
                        {renewable ? (
                            <>
                                <p css={tw`text-sm my-2`}>
                                    Seu servidor foi suspenso por não ser renovado a tempo. Clique no Renove o botão
                                    para reativar seu servidor. Se você quiser excluir Seu servidor, os recursos serão
                                    adicionados automaticamente à sua conta para que você pode implantar novamente um
                                    novo servidor facilmente.
                                </p>
                                <Button
                                    className={'mx-2 my-1'}
                                    onClick={() => setRenewDialog(true)}
                                    disabled={isSubmit}
                                >
                                    Renovar agora
                                </Button>
                                <Button.Danger
                                    className={'mx-2 my-1'}
                                    onClick={() => setDeleteDialog(true)}
                                    disabled={isSubmit}
                                >
                                    Excluir servidor
                                </Button.Danger>
                            </>
                        ) : (
                            <>
                                Este servidor está suspenso e não pode ser acessado. acesse:{' '}
                                <a className='text-blue-600' href='https://dash.seventyhost.net/'>
                                    https://dash.seventyhost.net/
                                </a>
                            </>
                        )}
                    </div>
                </div>
            </PageContentBlock>
        </>
    );
};
