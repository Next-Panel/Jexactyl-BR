import tw from 'twin.macro';
import { format } from 'date-fns';
import { breakpoint } from '@/theme';
import * as Icon from 'react-feather';
import styled from 'styled-components/macro';
import { useStoreState } from '@/state/hooks';
import React, { useState, useEffect } from 'react';
import { Button } from '@/components/elements/button';
import { Dialog } from '@/components/elements/dialog';
import GreyRowBox from '@/components/elements/GreyRowBox';
import ContentBox from '@/components/elements/ContentBox';
import useFlash, { useFlashKey } from '@/plugins/useFlash';
import deleteReferralCode from '@/api/account/deleteReferralCode';
import createReferralCode from '@/api/account/createReferralCode';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import PageContentBlock from '@/components/elements/PageContentBlock';
import getReferralCodes, { ReferralCode } from '@/api/account/getReferralCodes';
import getReferralActivity, { ReferralActivity } from '@/api/account/getReferralActivity';

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

export default () => {
    const { addFlash } = useFlash();
    const [code, setCode] = useState('');
    const [loading, setLoading] = useState(false);
    const [codes, setCodes] = useState<ReferralCode[]>([]);
    const [activity, setActivity] = useState<ReferralActivity[]>([]);
    const { clearFlashes, clearAndAddHttpError } = useFlashKey('referrals');
    const reward = useStoreState((state) => state.storefront.data?.referrals.reward);

    useEffect(() => {
        clearFlashes();
        setLoading(true);

        getReferralCodes()
            .then((codes) => {
                setCodes(codes);
                setLoading(false);
            })
            .catch((error) => clearAndAddHttpError(error));

        getReferralActivity()
            .then((activity) => {
                setActivity(activity);
                setLoading(false);
            })
            .catch((error) => clearAndAddHttpError(error));
    }, []);

    const doCreation = () => {
        clearFlashes();
        setLoading(true);

        createReferralCode()
            .then(() => {
                getReferralCodes().then((codes) => setCodes(codes));
                addFlash({
                    type: 'success',
                    key: 'referrals',
                    message: 'O código de referência foi criado.',
                });
            })
            .catch((error) => clearAndAddHttpError(error))
            .then(() => {
                setLoading(false);
            });
    };

    const doDeletion = (code: string) => {
        clearFlashes();
        setLoading(true);

        deleteReferralCode(code)
            .then(() => {
                getReferralCodes().then((codes) => setCodes(codes));
                addFlash({
                    type: 'success',
                    key: 'referrals',
                    message: 'O código de referência foi deletado.',
                });
            })
            .catch((error) => clearAndAddHttpError(error))
            .then(() => {
                setLoading(false);
                setCode('');
            });
    };

    return (
        <PageContentBlock
            title={'Indicações'}
            description={'Criar um código e compartilhá-lo com outros.'}
            showFlashKey={'referrals'}
        >
            <Container className={'j-up lg:grid lg:grid-cols-3 my-10'}>
                <ContentBox title={'Seus códigos de indicação'} css={tw`sm:mt-0`}>
                    <Dialog.Confirm
                        title={'Excluir Código de Indicação'}
                        confirm={'Excluir código'}
                        open={!!code}
                        onClose={() => setCode('')}
                        onConfirmed={() => doDeletion(code)}
                    >
                        Os usuários não poderão mais usar esta chave para se inscreverem.
                    </Dialog.Confirm>
                    <SpinnerOverlay visible={loading} />
                    {codes.length === 0 ? (
                        <p css={tw`text-center my-2`}>
                            {!loading && 'Não existem códigos de referência para esta conta.'}
                        </p>
                    ) : (
                        codes.map((code, index) => (
                            <GreyRowBox
                                key={code.code}
                                css={[tw`bg-neutral-900 flex items-center`, index > 0 && tw`mt-2`]}
                            >
                                <Icon.GitBranch css={tw`text-neutral-300`} />
                                <div css={tw`ml-4 flex-1 overflow-hidden`}>
                                    <p css={tw`text-sm break-words`}>{code.code}</p>
                                    <p css={tw`text-2xs text-neutral-300 uppercase`}>
                                        Criado em:&nbsp;
                                        {code.createdAt ? format(code.createdAt, 'MMM do, yyyy HH:mm') : 'Never'}
                                    </p>
                                </div>
                                <button css={tw`ml-4 p-2 text-sm`} onClick={() => setCode(code.code)}>
                                    <Icon.Trash
                                        css={tw`text-neutral-400 hover:text-red-400 transition-colors duration-150`}
                                    />
                                </button>
                            </GreyRowBox>
                        ))
                    )}
                    <Button onClick={() => doCreation()} className={'mt-4'}>
                        Criar
                    </Button>
                </ContentBox>
                <ContentBox title={'Vantagens Disponíveis'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <h1 css={tw`text-xl`}>
                        Você receberá <span className={'text-green-500'}>{reward}</span> créditos para cada usuário que
                        você indicar para este Painel.
                    </h1>
                </ContentBox>
                <ContentBox title={'Usuários indicados'} css={tw`mt-8 sm:mt-0 sm:ml-8`}>
                    <SpinnerOverlay visible={loading} />
                    {activity.length === 0 ? (
                        <p css={tw`text-center my-2`}>
                            {!loading && 'Não existe nenhuma atividade de indicação para esta conta.'}
                        </p>
                    ) : (
                        activity.map((act, index) => (
                            <GreyRowBox
                                key={act.code}
                                css={[tw`bg-neutral-900 flex items-center`, index > 0 && tw`mt-2`]}
                            >
                                <Icon.GitBranch css={tw`text-neutral-300`} />
                                <div css={tw`ml-4 flex-1 overflow-hidden`}>
                                    <p css={tw`text-sm break-words`}>
                                        {act.userEmail} (ID: {act.userId})
                                    </p>
                                    <p css={tw`text-2xs text-neutral-300 uppercase`}>
                                        Usado em:&nbsp;
                                        {act.createdAt ? format(act.createdAt, 'MMM do, yyyy HH:mm') : 'Nunca'}
                                    </p>
                                    <p css={tw`text-2xs text-neutral-300 uppercase`}>
                                        Usado em:&nbsp;{act.code}
                                    </p>
                                </div>
                            </GreyRowBox>
                        ))
                    )}
                </ContentBox>
            </Container>
        </PageContentBlock>
    );
};
