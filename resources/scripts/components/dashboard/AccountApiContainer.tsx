import tw from 'twin.macro';
import { format } from 'date-fns';
import { ptBR } from 'date-fns/locale';
import * as Icon from 'react-feather';
import Code from '@/components/elements/Code';
import { useFlashKey } from '@/plugins/useFlash';
import React, { useEffect, useState } from 'react';
import deleteApiKey from '@/api/account/deleteApiKey';
import { Dialog } from '@/components/elements/dialog';
import ContentBox from '@/components/elements/ContentBox';
import GreyRowBox from '@/components/elements/GreyRowBox';
import getApiKeys, { ApiKey } from '@/api/account/getApiKeys';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import PageContentBlock from '@/components/elements/PageContentBlock';
import CreateApiKeyForm from '@/components/dashboard/forms/CreateApiKeyForm';

export default () => {
    const [loading, setLoading] = useState(true);
    const [keys, setKeys] = useState<ApiKey[]>([]);
    const [deleteIdentifier, setDeleteIdentifier] = useState('');
    const { clearAndAddHttpError } = useFlashKey('account');

    useEffect(() => {
        getApiKeys()
            .then((keys) => setKeys(keys))
            .then(() => setLoading(false))
            .catch((error) => clearAndAddHttpError(error));
    }, []);

    const doDeletion = (identifier: string) => {
        setLoading(true);

        clearAndAddHttpError();
        deleteApiKey(identifier)
            .then(() => setKeys((s) => [...(s || []).filter((key) => key.identifier !== identifier)]))
            .catch((error) => clearAndAddHttpError(error))
            .then(() => {
                setLoading(false);
                setDeleteIdentifier('');
            });
    };

    return (
        <PageContentBlock
            title={'API da conta'}
            description={'Criar chaves API para interagir com o Painel.'}
            showFlashKey={'account'}
        >
            <div className={'j-up md:flex flex-nowrap my-10'}>
                <ContentBox title={'Criar Chave API'} css={tw`flex-none w-full md:w-1/2`}>
                    <CreateApiKeyForm onKeyCreated={(key) => setKeys((s) => [...s!, key])} />
                </ContentBox>
                <ContentBox title={'Chaves API'} css={tw`flex-1 overflow-hidden mt-8 md:mt-0 md:ml-8`}>
                    <SpinnerOverlay visible={loading} />
                    <Dialog.Confirm
                        title={'Deletar Chave API'}
                        confirm={'Deletar Chave'}
                        open={!!deleteIdentifier}
                        onClose={() => setDeleteIdentifier('')}
                        onConfirmed={() => doDeletion(deleteIdentifier)}
                    >
                        Todos os pedidos utilizando a chave <Code>{deleteIdentifier}</Code> será invalidado.
                    </Dialog.Confirm>
                    {keys.length === 0 ? (
                        <p css={tw`text-center text-sm`}>
                            {loading ? 'Carregando...' : 'Não existem chaves API para esta conta.'}
                        </p>
                    ) : (
                        keys.map((key, index) => (
                            <GreyRowBox
                                key={key.identifier}
                                css={[tw`bg-neutral-700 flex items-center`, index > 0 && tw`mt-2`]}
                            >
                                <Icon.Key css={tw`text-neutral-300`} />
                                <div css={tw`ml-4 flex-1 overflow-hidden`}>
                                    <p css={tw`text-sm break-words`}>{key.description}</p>
                                    <p css={tw`text-2xs text-neutral-300 uppercase`}>
                                        Última utilização:&nbsp;
                                        {key.lastUsedAt
                                            ? format(key.lastUsedAt, "'dia' d 'de' MMMM yyyy', ás' HH:mm", {
                                                  locale: ptBR,
                                              })
                                            : 'Nunca'}
                                    </p>
                                </div>
                                <p css={tw`text-sm ml-4 hidden md:block`}>
                                    <code css={tw`font-mono py-1 px-2 bg-neutral-900 rounded`}>{key.identifier}</code>
                                </p>
                                <button css={tw`ml-4 p-2 text-sm`} onClick={() => setDeleteIdentifier(key.identifier)}>
                                    <Icon.Trash
                                        css={tw`text-neutral-400 hover:text-red-400 transition-colors duration-150`}
                                    />
                                </button>
                            </GreyRowBox>
                        ))
                    )}
                </ContentBox>
            </div>
        </PageContentBlock>
    );
};
