import React from 'react';
import * as Yup from 'yup';
import { pt } from 'yup-locales';
import { ApplicationStore } from '@/state';
import { httpErrorToHuman } from '@/api/http';
import { useStoreState } from '@/state/hooks';
import Field from '@/components/elements/Field';
import { Form, Formik, FormikHelpers } from 'formik';
import { Actions, useStoreActions } from 'easy-peasy';
import { Button } from '@/components/elements/button/index';
import useReferralCode from '@/api/account/useReferralCode';

interface Values {
    code: string;
    password: string;
}

const schema = Yup.object().shape({
    code: Yup.string().length(16).required(),
    password: Yup.string().required('Deve fornecer a sua senha de conta corrente.'),
});

export default () => {
    const code = useStoreState((state) => state.user.data!.referralCode);
    const { clearFlashes, addFlash } = useStoreActions((actions: Actions<ApplicationStore>) => actions.flashes);
    Yup.setLocale(pt);

    const submit = (values: Values, { resetForm, setSubmitting }: FormikHelpers<Values>) => {
        clearFlashes('account:referral');

        useReferralCode({ ...values })
            .then(() =>
                addFlash({
                    type: 'success',
                    key: 'account:referral',
                    message: 'Agora você está usando o Código de referência.',
                }),
            )
            .catch((error) =>
                addFlash({
                    type: 'danger',
                    key: 'account:referral',
                    title: 'Erro',
                    message: httpErrorToHuman(error),
                }),
            )
            .then(() => {
                resetForm();
                setSubmitting(false);

                // @ts-expect-error this is valid
                window.location = '/account';
            });
    };

    return (
        <>
            {code ? (
                <p className={'my-2 text-gray-400'}>
                    Voce ja usou um código de referência
                    <span className={'bg-gray-800 rounded p-1 ml-2'}>{code}</span>
                </p>
            ) : (
                <Formik onSubmit={submit} initialValues={{ code: '', password: '' }} validationSchema={schema}>
                    {({ isSubmitting, isValid }) => (
                        <React.Fragment>
                            <Form className={'m-0'}>
                                <Field
                                    id={'code'}
                                    type={'text'}
                                    name={'code'}
                                    label={'Introduza o código de referência'}
                                />
                                <div className={'mt-6'}>
                                    <Field
                                        id={'confirm_password'}
                                        type={'password'}
                                        name={'password'}
                                        label={'Confirmar Senha'}
                                    />
                                </div>
                                <div className={'mt-6'}>
                                    <Button disabled={isSubmitting || !isValid}>Usar código</Button>
                                </div>
                            </Form>
                        </React.Fragment>
                    )}
                </Formik>
            )}
        </>
    );
};
