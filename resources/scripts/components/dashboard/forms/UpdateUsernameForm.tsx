import React from 'react';
import * as Yup from 'yup';
import { pt } from 'yup-locales';
import tw from 'twin.macro';
import { ApplicationStore } from '@/state';
import { httpErrorToHuman } from '@/api/http';
import Field from '@/components/elements/Field';
import { Form, Formik, FormikHelpers } from 'formik';
import { Actions, useStoreActions } from 'easy-peasy';
import { Button } from '@/components/elements/button/index';
import SpinnerOverlay from '@/components/elements/SpinnerOverlay';
import updateAccountUsername from '@/api/account/updateAccountUsername';

interface Values {
    username: string;
    password: string;
}

const schema = Yup.object().shape({
    username: Yup.string().min(3).required(),
    password: Yup.string().required('Deve fornecer a sua senha de conta corrente.'),
});

export default () => {
    const { clearFlashes, addFlash } = useStoreActions((actions: Actions<ApplicationStore>) => actions.flashes);

    Yup.setLocale(pt);

    const submit = (values: Values, { resetForm, setSubmitting }: FormikHelpers<Values>) => {
        clearFlashes('account:email');

        updateAccountUsername({ ...values })
            .then(() =>
                addFlash({
                    type: 'success',
                    key: 'account:username',
                    message: 'O seu usuário foi alterado.',
                }),
            )
            .catch((error) =>
                addFlash({
                    type: 'danger',
                    key: 'account:username',
                    title: 'Erro',
                    message: httpErrorToHuman(error),
                }),
            )
            .then(() => {
                resetForm();
                setSubmitting(false);
            });
    };

    return (
        <Formik onSubmit={submit} initialValues={{ username: '', password: '' }} validationSchema={schema}>
            {({ isSubmitting, isValid }) => (
                <React.Fragment>
                    <SpinnerOverlay size={'large'} visible={isSubmitting} />
                    <Form css={tw`m-0`}>
                        <Field id={'new_username'} type={'username'} name={'username'} label={'Novo Usúario'} />
                        <div css={tw`mt-6`}>
                            <Field
                                id={'confirm_password'}
                                type={'password'}
                                name={'password'}
                                label={'Confirmar Senha'}
                            />
                        </div>
                        <div css={tw`mt-6`}>
                            <Button disabled={isSubmitting || !isValid}>Atualizar Usúario</Button>
                        </div>
                    </Form>
                </React.Fragment>
            )}
        </Formik>
    );
};
